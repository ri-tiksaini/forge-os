#![forbid(unsafe_code)]

use forge_tier::{detect_from, Probes, Tier};

#[allow(clippy::too_many_arguments)]
fn machine(
    mem_gib: f64,
    cpu_model: &str,
    cpu_year: u16,
    rotational: bool,
    gpu_driver: &str,
    gles_major: u8,
    gles_minor: u8,
    sw: bool,
    has_drm: bool,
) -> Probes {
    Probes {
        mem_total_kb: (mem_gib * 1024.0 * 1024.0) as u64,
        cpu_model: cpu_model.to_string(),
        cpu_cores: 4,
        has_sse4_2: cpu_year >= 2010,
        has_avx2: cpu_year >= 2014,
        cpu_year,
        disk_rotational: rotational,
        disk_model: if rotational {
            "WDC 5400RPM".to_string()
        } else {
            "NVMe SSD".to_string()
        },
        gpu_vendor: "0x8086".to_string(),
        gpu_driver: gpu_driver.to_string(),
        gles_major,
        gles_minor,
        is_software_rasterizer: sw,
        has_drm_driver: has_drm,
        display_count: 1,
    }
}

#[test]
fn forty_real_world_machine_fixtures_match_expected_tiers() {
    let cases: [(&str, Probes, Tier); 40] = [
        // --- Tier 0: Legacy, low RAM, software rasterizer, no DRM (14 fixtures) ---
        (
            "01_thinkpad_x220_hdd",
            machine(4.0, "i5-2520M", 2011, true, "i915", 3, 0, false, true),
            Tier::Tier0,
        ),
        (
            "02_thinkpad_t420_2g",
            machine(2.0, "i5-2520M", 2011, true, "i915", 3, 0, false, true),
            Tier::Tier0,
        ),
        (
            "03_core2duo_2g_hdd",
            machine(
                2.0,
                "Core(TM)2 Duo P8600",
                2008,
                true,
                "i915",
                2,
                1,
                false,
                true,
            ),
            Tier::Tier0,
        ),
        (
            "04_core2duo_4g_ssd_gles2",
            machine(
                4.0,
                "Core(TM)2 Duo T9600",
                2008,
                false,
                "i915",
                2,
                1,
                false,
                true,
            ),
            Tier::Tier0,
        ),
        (
            "05_netbook_atom_2g",
            machine(2.0, "Atom N270", 2009, true, "i915", 2, 0, false, true),
            Tier::Tier0,
        ),
        (
            "06_qemu_tier0_2g_pixman",
            machine(2.0, "Westmere", 2011, true, "virtio-pci", 2, 0, true, true),
            Tier::Tier0,
        ),
        (
            "07_modern_cpu_3g_ram_override",
            machine(
                3.0,
                "Ryzen 7 7840U",
                2023,
                false,
                "amdgpu",
                3,
                2,
                false,
                true,
            ),
            Tier::Tier0,
        ),
        (
            "08_modern_cpu_no_drm_override",
            machine(16.0, "i7-1360P", 2023, false, "", 3, 2, false, false),
            Tier::Tier0,
        ),
        (
            "09_modern_cpu_llvmpipe_override",
            machine(16.0, "i7-1360P", 2023, false, "simpledrm", 3, 2, true, true),
            Tier::Tier0,
        ),
        (
            "10_dell_e6410_4g_hdd",
            machine(4.0, "i5-520M", 2010, true, "i915", 2, 1, false, true),
            Tier::Tier0,
        ),
        (
            "11_hp_2540p_3g_ssd",
            machine(3.2, "i7-640LM", 2010, false, "i915", 2, 1, false, true),
            Tier::Tier0,
        ),
        (
            "12_chromebook_2g_emmc",
            machine(2.0, "Celeron N3060", 2016, false, "i915", 3, 1, false, true),
            Tier::Tier0,
        ),
        (
            "13_macbook_2010_4g_nouveau",
            machine(
                4.0,
                "Core(TM)2 Duo",
                2010,
                true,
                "nouveau",
                2,
                1,
                false,
                true,
            ),
            Tier::Tier0,
        ),
        (
            "14_vga_vesa_fallback",
            machine(8.0, "i5-8250U", 2018, false, "vgem", 2, 0, true, true),
            Tier::Tier0,
        ),
        // --- Tier 1: Standard laptops & HDD-capped modern machines (14 fixtures) ---
        (
            "15_thinkpad_t480_8g_ssd",
            machine(8.0, "i5-8350U", 2018, false, "i915", 3, 1, false, true),
            Tier::Tier1,
        ),
        (
            "16_thinkpad_x230_8g_ssd",
            machine(8.0, "i5-3320M", 2012, false, "i915", 3, 0, false, true),
            Tier::Tier1,
        ),
        (
            "17_thinkpad_t440p_8g_ssd",
            machine(8.0, "i5-4300M", 2014, false, "i915", 3, 1, false, true),
            Tier::Tier1,
        ),
        (
            "18_dell_xps13_2015_8g",
            machine(8.0, "i5-5200U", 2015, false, "i915", 3, 1, false, true),
            Tier::Tier1,
        ),
        (
            "19_hp_elitebook_840g3_8g",
            machine(8.0, "i5-6300U", 2016, false, "i915", 3, 1, false, true),
            Tier::Tier1,
        ),
        (
            "20_lenovo_ideapad_4g_ssd",
            machine(4.0, "i3-8130U", 2018, false, "i915", 3, 1, false, true),
            Tier::Tier1,
        ),
        (
            "21_ryzen5_2500u_8g_ssd",
            machine(
                8.0,
                "Ryzen 5 2500U",
                2018,
                false,
                "amdgpu",
                3,
                1,
                false,
                true,
            ),
            Tier::Tier1,
        ),
        (
            "22_modern_desktop_16g_hdd_cap",
            machine(16.0, "i7-13700K", 2023, true, "amdgpu", 3, 2, false, true),
            Tier::Tier1,
        ),
        (
            "23_ryzen7_32g_hdd_cap",
            machine(
                32.0,
                "Ryzen 7 7700X",
                2023,
                true,
                "amdgpu",
                3,
                2,
                false,
                true,
            ),
            Tier::Tier1,
        ),
        (
            "24_surface_pro4_4g_ssd",
            machine(4.0, "i5-6300U", 2016, false, "i915", 3, 1, false, true),
            Tier::Tier1,
        ),
        (
            "25_thinkpad_t450s_12g_ssd",
            machine(12.0, "i5-5300U", 2015, false, "i915", 3, 1, false, true),
            Tier::Tier1,
        ),
        (
            "26_acer_swift3_8g_nvme",
            machine(8.0, "i5-1035G1", 2020, false, "i915", 3, 1, false, true),
            Tier::Tier1,
        ),
        (
            "27_thinkpad_x220_8g_ssd_upgrade",
            machine(8.0, "i5-2520M", 2011, false, "i915", 3, 0, false, true),
            Tier::Tier1,
        ),
        (
            "28_qemu_tier1_4g_ssd_virtio",
            machine(4.0, "Skylake", 2016, false, "virtio-pci", 3, 0, false, true),
            Tier::Tier1,
        ),
        // --- Tier 2: Modern SSD/NVMe workstations & laptops (12 fixtures) ---
        (
            "29_framework_13_amd_16g",
            machine(
                16.0,
                "Ryzen 7 7840U",
                2023,
                false,
                "amdgpu",
                3,
                2,
                false,
                true,
            ),
            Tier::Tier2,
        ),
        (
            "30_thinkpad_t14_gen4_32g",
            machine(32.0, "i7-1365U", 2023, false, "i915", 3, 2, false, true),
            Tier::Tier2,
        ),
        (
            "31_dell_xps15_2024_32g",
            machine(
                32.0,
                "Core Ultra 7 155H",
                2024,
                false,
                "xe",
                3,
                2,
                false,
                true,
            ),
            Tier::Tier2,
        ),
        (
            "32_ryzen9_workstation_64g",
            machine(
                64.0,
                "Ryzen 9 7950X",
                2023,
                false,
                "amdgpu",
                3,
                2,
                false,
                true,
            ),
            Tier::Tier2,
        ),
        (
            "33_thinkpad_p1_16g_nvme",
            machine(16.0, "i7-12800H", 2022, false, "i915", 3, 2, false, true),
            Tier::Tier2,
        ),
        (
            "34_system76_lemur_16g",
            machine(16.0, "i5-1335U", 2023, false, "i915", 3, 2, false, true),
            Tier::Tier2,
        ),
        (
            "35_asus_zenbook_16g_amd",
            machine(
                16.0,
                "Ryzen 5 7535U",
                2023,
                false,
                "amdgpu",
                3,
                2,
                false,
                true,
            ),
            Tier::Tier2,
        ),
        (
            "36_hp_dev_one_16g",
            machine(
                16.0,
                "Ryzen 7 5850U",
                2021,
                false,
                "amdgpu",
                3,
                2,
                false,
                true,
            ),
            Tier::Tier2,
        ),
        (
            "37_tuxedo_pulse_32g",
            machine(
                32.0,
                "Ryzen 7 8845HS",
                2024,
                false,
                "amdgpu",
                3,
                2,
                false,
                true,
            ),
            Tier::Tier2,
        ),
        (
            "38_intel_nuc13_16g",
            machine(16.0, "i5-1340P", 2023, false, "i915", 3, 2, false, true),
            Tier::Tier2,
        ),
        (
            "39_desktop_i5_12400_16g",
            machine(16.0, "i5-12400", 2022, false, "amdgpu", 3, 2, false, true),
            Tier::Tier2,
        ),
        (
            "40_thinkpad_x1c_g11_16g",
            machine(16.0, "i7-1355U", 2023, false, "i915", 3, 2, false, true),
            Tier::Tier2,
        ),
    ];

    for (name, probes, expected) in cases {
        let report = detect_from(&probes);
        assert_eq!(
            report.tier, expected,
            "fixture {name} expected {expected:?}, got {:?} (score={}, reasons={:?})",
            report.tier, report.score, report.reasons
        );
    }
}

#[test]
fn no_drm_sets_nomodeset_flag() {
    let p = machine(16.0, "i7-1360P", 2023, false, "", 3, 2, false, false);
    let r = detect_from(&p);
    assert_eq!(r.tier, Tier::Tier0);
    assert!(r.nomodeset_recommended);
}
