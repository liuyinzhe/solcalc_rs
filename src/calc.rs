/// Unit multipliers relative to base units (g, L, mol/L)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MassUnit {
    G,
    Mg,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolumeUnit {
    L,
    ML,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConcUnit {
    MolPerL,
    MmolPerL,
    UmolPerL,
    MgPerMl,
    UgPerMl,
    GPerL,
}

impl MassUnit {
    pub fn to_g(self, v: f64) -> f64 {
        match self {
            MassUnit::G => v,
            MassUnit::Mg => v / 1000.0,
        }
    }
}

impl VolumeUnit {
    pub fn to_l(self, v: f64) -> f64 {
        match self {
            VolumeUnit::L => v,
            VolumeUnit::ML => v / 1000.0,
        }
    }
}

impl ConcUnit {
    pub fn to_mol_per_l(self, v: f64, molar_mass_g_per_mol: Option<f64>) -> Option<f64> {
        match self {
            ConcUnit::MolPerL => Some(v),
            ConcUnit::MmolPerL => Some(v / 1000.0),
            ConcUnit::UmolPerL => Some(v / 1_000_000.0),
            ConcUnit::MgPerMl => {
                let m = molar_mass_g_per_mol?;
                Some(v / m) // mg/mL = g/L, divide by g/mol → mol/L
            }
            ConcUnit::UgPerMl => {
                let m = molar_mass_g_per_mol?;
                Some(v / 1000.0 / m)
            }
            ConcUnit::GPerL => {
                let m = molar_mass_g_per_mol?;
                Some(v / m)
            }
        }
    }
}

/// Core calculation result for current solution
#[derive(Debug, Clone)]
pub struct SolutionInfo {
    pub molar_conc_mol_per_l: f64,
    pub mass_vol_conc_mg_per_ml: f64,
}

/// Result of dilution/concentration adjustment
#[derive(Debug, Clone)]
pub struct AdjustResult {
    /// Volume of solvent to add (L), Some only when target < current
    pub solvent_to_add_l: Option<f64>,
    /// Mass of solute to add (g), Some only when target > current
    pub solute_to_add_g: Option<f64>,
}

/// Calculate current solution concentrations.
/// All inputs in base units: molar_mass g/mol, mass g, volume L.
pub fn calc_solution(molar_mass: f64, mass_g: f64, volume_l: f64) -> Option<SolutionInfo> {
    if molar_mass <= 0.0 || mass_g <= 0.0 || volume_l <= 0.0 {
        return None;
    }
    let molar_conc = mass_g / (molar_mass * volume_l);
    let mass_vol = (mass_g * 1000.0) / (volume_l * 1000.0); // g/L → mg/mL = same ratio
    Some(SolutionInfo {
        molar_conc_mol_per_l: molar_conc,
        mass_vol_conc_mg_per_ml: mass_vol,
    })
}

/// Calculate how much solvent or solute to add to reach target_mol_per_l.
pub fn calc_adjust(
    current: &SolutionInfo,
    volume_l: f64,
    molar_mass: f64,
    target_mol_per_l: f64,
) -> AdjustResult {
    let c1 = current.molar_conc_mol_per_l;
    let solvent = if target_mol_per_l < c1 && target_mol_per_l > 0.0 {
        Some(volume_l * (c1 / target_mol_per_l - 1.0))
    } else {
        None
    };
    let solute = if target_mol_per_l > c1 {
        Some((target_mol_per_l - c1) * volume_l * molar_mass)
    } else {
        None
    };
    AdjustResult {
        solvent_to_add_l: solvent,
        solute_to_add_g: solute,
    }
}

/// Format a float to 4 significant figures, using scientific notation when needed.
pub fn fmt_sig4(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let mag = v.abs().log10().floor() as i32;
    if mag >= -2 && mag <= 5 {
        let decimals = (3 - mag).max(0) as usize;
        format!("{:.prec$}", v, prec = decimals)
    } else {
        format!("{:.3e}", v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_concentration() {
        // NaCl: M=58.44, m=5.844g, V=1L → c=0.1 mol/L, 5.844 mg/mL
        let info = calc_solution(58.44, 5.844, 1.0).unwrap();
        assert!((info.molar_conc_mol_per_l - 0.1).abs() < 1e-6);
        assert!((info.mass_vol_conc_mg_per_ml - 5.844).abs() < 1e-4);
    }

    #[test]
    fn test_dilution() {
        // 0.1 mol/L, 1L → target 0.05 mol/L: add 1L solvent
        let info = calc_solution(58.44, 5.844, 1.0).unwrap();
        let adj = calc_adjust(&info, 1.0, 58.44, 0.05);
        let sv = adj.solvent_to_add_l.unwrap();
        assert!((sv - 1.0).abs() < 1e-6, "expected 1.0 L, got {}", sv);
        assert!(adj.solute_to_add_g.is_none());
    }

    #[test]
    fn test_concentration_increase() {
        // 0.1 mol/L, 1L → target 0.2 mol/L, M=58.44: add 5.844g
        let info = calc_solution(58.44, 5.844, 1.0).unwrap();
        let adj = calc_adjust(&info, 1.0, 58.44, 0.2);
        let sm = adj.solute_to_add_g.unwrap();
        assert!((sm - 5.844).abs() < 1e-4, "expected 5.844g, got {}", sm);
        assert!(adj.solvent_to_add_l.is_none());
    }

    #[test]
    fn test_invalid_inputs() {
        assert!(calc_solution(0.0, 5.0, 1.0).is_none());
        assert!(calc_solution(58.44, -1.0, 1.0).is_none());
        assert!(calc_solution(58.44, 5.0, 0.0).is_none());
    }

    #[test]
    fn test_unit_conversion_mass() {
        assert_eq!(MassUnit::Mg.to_g(1000.0), 1.0);
        assert_eq!(MassUnit::G.to_g(1.0), 1.0);
    }

    #[test]
    fn test_unit_conversion_volume() {
        assert_eq!(VolumeUnit::ML.to_l(1000.0), 1.0);
        assert_eq!(VolumeUnit::L.to_l(1.0), 1.0);
    }

    #[test]
    fn test_conc_unit_mg_per_ml() {
        // 5.844 mg/mL with M=58.44 → 0.1 mol/L
        let c = ConcUnit::MgPerMl.to_mol_per_l(5.844, Some(58.44)).unwrap();
        assert!((c - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_fmt_sig4() {
        assert_eq!(fmt_sig4(0.1), "0.1000");
        assert_eq!(fmt_sig4(5.844), "5.844");
        assert_eq!(fmt_sig4(1000.0), "1000");
        assert_eq!(fmt_sig4(1e-5), "1.000e-5");
    }
}
