#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod calc;
use calc::{calc_adjust, calc_solution, fmt_sig4, ConcUnit, MassUnit, VolumeUnit};

slint::include_modules!();

fn parse_positive(s: &str) -> Option<f64> {
    let v: f64 = s.trim().parse().ok()?;
    if v > 0.0 { Some(v) } else { None }
}

fn mass_unit(idx: i32) -> MassUnit {
    if idx == 1 { MassUnit::Mg } else { MassUnit::G }
}

fn volume_unit(idx: i32) -> VolumeUnit {
    if idx == 1 { VolumeUnit::ML } else { VolumeUnit::L }
}

fn conc_unit(idx: i32) -> ConcUnit {
    match idx {
        1 => ConcUnit::MmolPerL,
        2 => ConcUnit::UmolPerL,
        3 => ConcUnit::MgPerMl,
        4 => ConcUnit::UgPerMl,
        5 => ConcUnit::GPerL,
        _ => ConcUnit::MolPerL,
    }
}

fn recalculate(ui: &AppWindow) {
    let mm_str = ui.get_molar_mass_text().to_string();
    let m_str  = ui.get_mass_text().to_string();
    let v_str  = ui.get_volume_text().to_string();

    let mm = parse_positive(&mm_str);
    let mass_raw = parse_positive(&m_str);
    let vol_raw  = parse_positive(&v_str);

    let mass_g = mass_raw.map(|v| mass_unit(ui.get_mass_unit_index()).to_g(v));
    let vol_l  = vol_raw.map(|v| volume_unit(ui.get_volume_unit_index()).to_l(v));

    // Validate inputs
    let has_input = !mm_str.trim().is_empty()
        || !m_str.trim().is_empty()
        || !v_str.trim().is_empty();

    let error = if has_input {
        let mm_bad  = !mm_str.trim().is_empty() && mm.is_none();
        let m_bad   = !m_str.trim().is_empty()  && mass_raw.is_none();
        let v_bad   = !v_str.trim().is_empty()  && vol_raw.is_none();
        if mm_bad || m_bad || v_bad {
            "⚠ 请输入有效的正数".to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    ui.set_error_msg(error.into());

    // Current solution concentrations
    match (mm, mass_g, vol_l) {
        (Some(m), Some(mg), Some(vl)) => {
            match calc_solution(m, mg, vl) {
                Some(info) => {
                    ui.set_molar_conc_result(fmt_sig4(info.molar_conc_mol_per_l).into());
                    ui.set_mass_vol_result(fmt_sig4(info.mass_vol_conc_mg_per_ml).into());

                    // Target concentration
                    let tc_str = ui.get_target_conc_text().to_string();
                    if let Some(tc_raw) = parse_positive(&tc_str) {
                        let tc_unit = conc_unit(ui.get_target_unit_index());
                        match tc_unit.to_mol_per_l(tc_raw, Some(m)) {
                            Some(tc_mol) => {
                                let adj = calc_adjust(&info, vl, m, tc_mol);
                                match adj.solvent_to_add_l {
                                    Some(sv) => {
                                        let (val, unit) = if sv >= 1.0 {
                                            (fmt_sig4(sv), "L")
                                        } else {
                                            (fmt_sig4(sv * 1000.0), "mL")
                                        };
                                        ui.set_solvent_result(format!("{} {}", val, unit).into());
                                    }
                                    None => ui.set_solvent_result("目标浓度 ≥ 当前浓度，无需稀释".into()),
                                }
                                match adj.solute_to_add_g {
                                    Some(sm) => {
                                        let (val, unit) = if sm >= 1.0 {
                                            (fmt_sig4(sm), "g")
                                        } else {
                                            (fmt_sig4(sm * 1000.0), "mg")
                                        };
                                        ui.set_solute_result(format!("{} {}", val, unit).into());
                                    }
                                    None => ui.set_solute_result("目标浓度 ≤ 当前浓度，无需增浓".into()),
                                }
                            }
                            None => {
                                ui.set_solvent_result("需要摩尔质量才能换算".into());
                                ui.set_solute_result("需要摩尔质量才能换算".into());
                            }
                        }
                    } else {
                        ui.set_solvent_result("—".into());
                        ui.set_solute_result("—".into());
                    }
                }
                None => {
                    ui.set_molar_conc_result("—".into());
                    ui.set_mass_vol_result("—".into());
                }
            }
        }
        _ => {
            if mm.is_none() && mass_g.is_none() && vol_l.is_none() {
                ui.set_molar_conc_result("—".into());
                ui.set_mass_vol_result("—".into());
                ui.set_solvent_result("—".into());
                ui.set_solute_result("—".into());
            }
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let ui_weak = ui.as_weak();
    ui.on_inputs_changed(move || {
        if let Some(ui) = ui_weak.upgrade() {
            recalculate(&ui);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_reset_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_molar_mass_text("".into());
            ui.set_mass_text("".into());
            ui.set_volume_text("".into());
            ui.set_target_conc_text("".into());
            ui.set_mass_unit_index(0);
            ui.set_volume_unit_index(0);
            ui.set_target_unit_index(0);
            ui.set_molar_conc_result("—".into());
            ui.set_mass_vol_result("—".into());
            ui.set_solvent_result("—".into());
            ui.set_solute_result("—".into());
            ui.set_error_msg("".into());
        }
    });

    ui.run()
}
