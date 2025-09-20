use super::RazerGuiApp;
use librazer::types::{CpuBoost, GpuBoost, PerfMode};
use strum::IntoEnumIterator;

impl RazerGuiApp {
    pub fn perf_mode_to_string(mode: PerfMode) -> String {
        format!("{:?}", mode)
    }
    pub fn string_to_perf_mode(mode: &str) -> Option<PerfMode> {
        PerfMode::iter().find(|m| format!("{:?}", m) == mode)
    }

    pub fn detect_available_performance_modes(&mut self) {
        if let Some(ref device) = self.device {
            if let Some(list) = device.info().perf_modes {
                self.available_performance_modes = list.to_vec();
                if self.base_performance_modes.is_empty() {
                    self.base_performance_modes = self.available_performance_modes.clone();
                }
                return;
            }
        }
        self.available_performance_modes = PerfMode::iter().collect();
        if self.base_performance_modes.is_empty() {
            self.base_performance_modes = self.available_performance_modes.clone();
        }
    }

    pub fn get_descriptor_allowed_boosts(
        &self,
    ) -> (Vec<CpuBoost>, Vec<GpuBoost>, Vec<(CpuBoost, GpuBoost)>) {
        if let Some(ref device) = self.device {
            let d = device.info();
            let cpus: Vec<CpuBoost> = d.cpu_boosts.map(|s| s.to_vec()).unwrap_or_else(|| {
                vec![CpuBoost::Low, CpuBoost::Medium, CpuBoost::High, CpuBoost::Boost]
            });
            let gpus: Vec<GpuBoost> = d
                .gpu_boosts
                .map(|s| s.to_vec())
                .unwrap_or_else(|| vec![GpuBoost::Low, GpuBoost::Medium, GpuBoost::High]);
            let pairs: Vec<(CpuBoost, GpuBoost)> =
                d.disallowed_boost_pairs.map(|p| p.to_vec()).unwrap_or_default();
            (cpus, gpus, pairs)
        } else {
            (
                vec![CpuBoost::Low, CpuBoost::Medium, CpuBoost::High, CpuBoost::Boost],
                vec![GpuBoost::Low, GpuBoost::Medium, GpuBoost::High],
                Vec::new(),
            )
        }
    }
}
