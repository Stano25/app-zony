use regex::Regex;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs::{self, File},
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf}, u16,
};
use std::hash::{Hash, Hasher};
use geo::{Point as GeoPoint,Geodesic, Distance};
use crate::managers::position_manager::lat_to_zone_letter;

use super::position_manager::{get_zone_from_lon, to_utm_wgs84, wsg84_utm_to_lat_lon};
use super::excel_manager::{update_excel_cell, get_positions_of_table,get_rows_of_table, fill_row, fill_row_db, update_excel_cell_smart, write_measurements_to_excel, write_measurements_to_excel_mobile};

const CITY_CELL: &str = "Meraná obec:"; // Veľkosť mriežky v metroch
const DATE_CELL: &str = "Dátum merania:";
const ANTENNA_CELL: &str = "Korekcia výšky antény (dB):";
const ENVIRONMENT_CELL: &str = "Korekcia vnútorného prostredia (dB):";
const GSM_TABLE_START_CELL: &str = "GSM";
const LTE_TABLE_START_CELL: &str = "LTE";
const NR_5G_TABLE_START_CELL: &str = "5G NR";
const ZONES_PEOPLE_CELL: &str = "Celkový počet zón 100x100m/obyvateľov:";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LteRecord {
    #[serde(rename = "Date")]
    pub date: String,

    #[serde(rename = "Time")]
    pub time: String,

    #[serde(rename = "UTC")]
    pub utc: Option<i64>, // Vrátené na i64 (celé číslo)

    #[serde(rename = "Latitude")]
    pub latitude: Option<f64>,

    #[serde(rename = "Longitude")]
    pub longitude: Option<f64>,

    #[serde(rename = "Altitude")]
    pub altitude: Option<f64>,

    #[serde(rename = "Speed")]
    pub speed: Option<f32>,

    #[serde(rename = "Heading")]
    pub heading: Option<f32>,

    #[serde(rename = "#Sat")]
    pub num_sat: Option<u8>,

    #[serde(rename = "EARFCN")]
    pub earfcn: Option<u32>, 

    #[serde(rename = "Frequency")]
    pub frequency: Option<u64>,

    #[serde(rename = "PCI")]
    pub pci: Option<u16>,

    #[serde(rename = "MCC")]
    pub mcc: Option<u16>,

    #[serde(rename = "MNC")]
    pub mnc: Option<u16>,

    #[serde(rename = "TAC")]
    pub tac: Option<u32>,

    #[serde(rename = "CI")]
    pub ci: Option<u32>,

    #[serde(rename = "eNodeB-ID")]
    pub enodeb_id: Option<u32>,

    #[serde(rename = "cellID")]
    pub cell_id: Option<u16>,

    #[serde(rename = "BW")]
    pub bandwidth: Option<f32>,

    #[serde(rename = "SymPerSlot")]
    pub sym_per_slot: Option<u8>,

    #[serde(rename = "Power")]
    pub power: Option<f64>,

    #[serde(rename = "SINR")]
    pub sinr: Option<f64>,

    #[serde(rename = "RSRP")]
    pub rsrp: Option<f64>,

    #[serde(rename = "RSRQ")]
    pub rsrq: Option<f32>,

    #[serde(rename = "4G-Drift")]
    pub drift_4g: Option<f32>,

    #[serde(rename = "Sigma-4G-Drift")]
    pub sigma_drift_4g: Option<f32>,

    #[serde(rename = "TimeOfArrival")]
    pub time_of_arrival: Option<f64>,

    #[serde(rename = "TimeOfArrivalFN")]
    pub time_of_arrival_fn: Option<u32>,

    #[serde(rename = "LTE-M")]
    pub lte_m: String, // Odstránené Option (prázdne bude "")

    #[serde(rename = "5G NR")]
    pub nr_5g: String, // Odstránené Option

    #[serde(rename = "eNodeB Tx Ports")]
    pub enodeb_tx_ports: Option<u8>,

    #[serde(rename = "SIB2 eMBMS/DSS")]
    pub sib2_embms_dss: String, // Odstránené Option

    #[serde(rename = "MIB dl_Bandwidth(MHz)")]
    pub mib_dl_bandwidth: Option<f32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GsmRecord {
    #[serde(rename = "Date")]
    pub date: String, // String nemusí byť Option, prázdna bunka bude proste ""

    #[serde(rename = "Time")]
    pub time: String,

    #[serde(rename = "UTC")]
    pub utc: Option<i64>, 

    #[serde(rename = "Latitude")]
    pub latitude: Option<f64>,

    #[serde(rename = "Longitude")]
    pub longitude: Option<f64>,

    #[serde(rename = "Altitude")]
    pub altitude: Option<f64>,

    #[serde(rename = "Speed")]
    pub speed: Option<f32>,

    #[serde(rename = "Heading")]
    pub heading: Option<f32>,

    #[serde(rename = "#Sat")]
    pub num_sat: Option<u8>,

    #[serde(rename = "ARFCN")]
    pub arfcn: Option<u16>,

    #[serde(rename = "Frequency")]
    pub frequency: Option<u64>,

    #[serde(rename = "MCC")]
    pub mcc: Option<u16>,

    #[serde(rename = "MNC")]
    pub mnc: Option<u16>,

    #[serde(rename = "LAC")]
    pub lac: Option<u32>,

    #[serde(rename = "CI")]
    pub ci: Option<u32>,

    #[serde(rename = "BSIC")]
    pub bsic: Option<u8>,

    #[serde(rename = "TotalPower")]
    pub total_power: Option<f64>,

    #[serde(rename = "SCHPower")]
    pub sch_power: Option<f64>,

    #[serde(rename = "C2I")]
    pub c2i: Option<f32>,

    #[serde(rename = "DeviceID")]
    pub device_id: Option<u16>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FiveGRecord {
    #[serde(rename = "Date")]
    pub date: String,

    #[serde(rename = "Time")]
    pub time: String,

    #[serde(rename = "UTC")]
    pub utc: Option<i64>,

    #[serde(rename = "Latitude")]
    pub latitude: Option<f64>,

    #[serde(rename = "Longitude")]
    pub longitude: Option<f64>,

    #[serde(rename = "Altitude")]
    pub altitude: Option<f64>,

    #[serde(rename = "Speed")]
    pub speed: Option<f32>,

    #[serde(rename = "Heading")]
    pub heading: Option<f32>,

    #[serde(rename = "#Sat")]
    pub num_sat: Option<u8>,

    #[serde(rename = "NR-ARFCN")]
    pub nr_arfcn: Option<u32>, // 5G ARFCN čísla sú väčšie, preto u32

    #[serde(rename = "SSRef")]
    pub ss_ref: Option<u64>, // Frekvencia v Hz, veľké číslo

    #[serde(rename = "Band")]
    pub band: Option<u16>,

    #[serde(rename = "PCI")]
    pub pci: Option<u16>,

    #[serde(rename = "SSB Idx")]
    pub ssb_idx: Option<u8>,

    #[serde(rename = "SSB Idx Mod8")]
    pub ssb_idx_mod8: Option<u8>,

    #[serde(rename = "SSB-RSSI")]
    pub ssb_rssi: Option<f64>,

    #[serde(rename = "SSS-SINR")]
    pub sss_sinr: Option<f64>,

    #[serde(rename = "SSS-RSRP")]
    pub sss_rsrp: Option<f64>,

    #[serde(rename = "SSS-RSRQ")]
    pub sss_rsrq: Option<f64>,

    #[serde(rename = "SSS-RePower")]
    pub sss_re_power: Option<f64>,

    #[serde(rename = "MCC")]
    pub mcc: Option<u16>,

    #[serde(rename = "MNC")]
    pub mnc: Option<u16>,

    #[serde(rename = "LAC")]
    pub lac: Option<u32>,

    #[serde(rename = "RNC-CellID(H)")]
    pub rnc_cell_id_h: Option<String>,

    // Podľa obrázku hodnota obsahuje pomlčku (napr. "300-2064516"), musí byť String
    #[serde(rename = "RNC-CellID(D)")]
    pub rnc_cell_id_d: Option<String>,

    #[serde(rename = "ToA(PPS)")]
    pub toa_pps: Option<f64>,

    #[serde(rename = "ToA(CIR)")]
    pub toa_cir: Option<f64>,

    #[serde(rename = "MIB_Sfn")]
    pub mib_sfn: Option<u32>,

    #[serde(rename = "MIB_ScsCommon30or120kHz")]
    pub mib_scs_common: Option<String>, // Hodnota je napr. "scs15or60"

    #[serde(rename = "MIB_SsbSubcarrierOffset")]
    pub mib_ssb_subcarrier_offset: Option<u16>,

    #[serde(rename = "MIB_DmrsTypeAPositionPos3")]
    pub mib_dmrs_type_a_pos3: Option<String>, // Hodnota je napr. "pos2"

    #[serde(rename = "MIB_PdcchConfigSib1")]
    pub mib_pdcch_config_sib1: Option<u32>,

    #[serde(rename = "MIB_CellNotBarred")]
    pub mib_cell_not_barred: Option<String>, // Hodnota je "notBarred"

    #[serde(rename = "MIB_IntraFreqReselectionNotAllowed")]
    pub mib_intra_freq_reselection: Option<String>, // Hodnota je "allowed"

    #[serde(rename = "DM_RS-SINR")]
    pub dm_rs_sinr: Option<f64>,

    #[serde(rename = "DM_RS-RSRP")]
    pub dm_rs_rsrp: Option<f64>,

    #[serde(rename = "DM_RS-RSRQ")]
    pub dm_rs_rsrq: Option<f64>,

    #[serde(rename = "DM_RS-RePower")]
    pub dm_rs_re_power: Option<f64>,

    #[serde(rename = "PBCH-SINR")]
    pub pbch_sinr: Option<f64>,

    #[serde(rename = "PBCH-RSRP")]
    pub pbch_rsrp: Option<f64>,

    #[serde(rename = "PBCH-RSRQ")]
    pub pbch_rsrq: Option<f64>,

    #[serde(rename = "PBCH-RePower")]
    pub pbch_re_power: Option<f64>,

    #[serde(rename = "PSS-SINR")]
    pub pss_sinr: Option<f64>,

    #[serde(rename = "PSS-RSRP")]
    pub pss_rsrp: Option<f64>,

    #[serde(rename = "PSS-RSRQ")]
    pub pss_rsrq: Option<f64>,

    #[serde(rename = "PSS-RePower")]
    pub pss_re_power: Option<f64>,

    #[serde(rename = "SSS_PBCH-SINR")]
    pub sss_pbch_sinr: Option<f64>,

    #[serde(rename = "SSS_PBCH-RSRP")]
    pub sss_pbch_rsrp: Option<f64>,

    #[serde(rename = "SSS_PBCH-RSRQ")]
    pub sss_pbch_rsrq: Option<f64>,

    #[serde(rename = "SSS_PBCH-RePower")]
    pub sss_pbch_re_power: Option<f64>,

    #[serde(rename = "SS_PBCH-SINR")]
    pub ss_pbch_sinr: Option<f64>,

    #[serde(rename = "SS_PBCH-RSRP")]
    pub ss_pbch_rsrp: Option<f64>,

    #[serde(rename = "SS_PBCH-RSRQ")]
    pub ss_pbch_rsrq: Option<f64>,

    #[serde(rename = "SS_PBCH-RePower")]
    pub ss_pbch_re_power: Option<f64>,

    #[serde(rename = "PSS_CI-DtoL")]
    pub pss_ci_dtol: Option<f64>,

    #[serde(rename = "PSS_CI-DtoH")]
    pub pss_ci_dtoh: Option<f64>,

    #[serde(rename = "SSS_CI-DtoL")]
    pub sss_ci_dtol: Option<f64>,

    #[serde(rename = "SSS_CI-DtoH")]
    pub sss_ci_dtoh: Option<f64>,

    #[serde(rename = "DeviceID")]
    pub device_id: Option<u16>,

    #[serde(rename = "Add. PLMNs")]
    pub add_plmns: Option<String>,
}

struct MncRule {
    target_mcc: u16,
    target_mnc: u16,
    freq_ranges: Vec<(u64, u64)>, // (start, end)
}

pub trait RecordValidator {
    fn is_valid(&self) -> bool;
}

pub trait RecordFilter: Clone + Sized {

    // Getters
    fn get_mcc(&self) -> Option<u16>;
    fn get_mnc(&self) -> Option<u16>;
    fn get_freq(&self) -> Option<u64>;
    fn get_lat(&self) -> Option<f64>; // Potrebujeme pre grid
    fn get_lon(&self) -> Option<f64>; // Potrebujeme pre grid
    fn get_date(&self) -> String;
    fn get_total_power(&self) -> Option<f64>;
    fn get_rsrp(&self) -> Option<f64>;
    fn get_sinr(&self) -> Option<f64>;
    fn get_5g_nr(&self) -> Option<String>;
    fn get_pci(&self) -> Option<u16>;
    fn get_time(&self) -> String;

    // Setters (pre duplikáciu záznamov)
    fn set_lat(&mut self, val: f64);
    fn set_lon(&mut self, val: f64);
    fn set_mcc(&mut self, val: u16);
    fn set_mnc(&mut self, val: u16);

    fn get_signal_strength(&self) -> f64; 

    // Vytvorí jeden spriemerovaný záznam zo zoznamu
    fn create_summary(records: &[Self]) -> Self;

    fn create_dummy(&self, mcc: u16, mnc: u16) -> Self;

    fn apply_custom_filter(records: Vec<Self>, filter_files: Option<Vec<PathBuf>>) -> Result<Vec<Self>, Box<dyn Error>> {
        // Predvolená implementácia (ak by nejaký struct filtre nepotreboval) vráti nezmenené dáta
        Ok(records)
    }
}

pub trait PointComparation: Clone {
    fn is_secondary_above_threshold(&self, threshold: f32) -> bool;
    fn is_primary_above_threshold_and_above_old(&self, threshold: f32, old_value: f64) -> bool;
}

fn has_5g_nr_yes(value: Option<&str>) -> bool {
    match value {
        Some(raw) => {
            let clean = raw.trim().to_ascii_lowercase();
            clean == "yes" || clean == "true" || clean == "1"
        }
        None => false,
    }
}

impl RecordFilter for LteRecord {
    fn get_mcc(&self) -> Option<u16> { self.mcc }
    fn get_mnc(&self) -> Option<u16> { self.mnc }
    fn get_freq(&self) -> Option<u64> { self.frequency }
    fn get_lat(&self) -> Option<f64> { self.latitude }
    fn get_lon(&self) -> Option<f64> { self.longitude }
    fn get_date(&self) -> String { self.date.clone() }
    fn get_total_power(&self) -> Option<f64> { None }
    fn get_rsrp(&self) -> Option<f64> { self.rsrp }
    fn get_sinr(&self) -> Option<f64> { self.sinr }
    fn get_5g_nr(&self) -> Option<String> { Some(self.nr_5g.clone()) }
    fn set_lat(&mut self, val: f64) {self.latitude = Some(val);}
    fn set_lon(&mut self, val: f64) {self.longitude = Some(val);}
    fn set_mcc(&mut self, val: u16) { self.mcc = Some(val); }
    fn set_mnc(&mut self, val: u16) { self.mnc = Some(val); }
    fn get_time(&self) -> String { self.time.clone() }
    fn get_pci(&self) -> Option<u16> {
        self.pci
    }

    fn get_signal_strength(&self) -> f64 {
        self.rsrp.unwrap_or(-999.0)
    }

    // Implementácia logiky priemerovania (kópia z tvojej funkcie)
    fn create_summary(records: &[Self]) -> Self {
            // Ak je prázdny, vráti "prázdny" klon (teoreticky by sa nemalo stať)
            if records.is_empty() { return records[0].clone(); }

            let first = &records[0];
            
            // 1. Fixné hodnoty (tie sú rovnaké pre celú skupinu)
            let fixed_mcc = first.mcc;
            let fixed_mnc = first.mnc;
            let fixed_freq = first.frequency;

            // 2. Reprezentatívne hodnoty (prvé nenulové)
            let first_date = records.iter().find(|r| !r.date.is_empty()).map(|r| r.date.clone()).unwrap_or_default();
            let first_time = records.iter().find(|r| !r.time.is_empty()).map(|r| r.time.clone()).unwrap_or_default();
            
            // ... ostatné first_ veci (skrátený výpis, doplň podľa tvojej funkcie)
            let first_utc = records.iter().find_map(|r| r.utc);
            let first_lat = records.first().and_then(|r| r.latitude);
            let first_lon = records.first().and_then(|r| r.longitude);
            // Doplň zvyšok find_map pre všetky polia...
            
            // 3. Výpočet priemerov
            let count = records.len() as f64;
            
            let sum_rsrp: f64 = records.iter().filter_map(|r| r.rsrp).sum();
            let avg_rsrp = if count > 0.0 { (sum_rsrp / count * 100.0).round() / 100.0 } else { 0.0 };

            let sum_sinr: f64 = records.iter().filter_map(|r| r.sinr).sum();
            let avg_sinr = if count > 0.0 { (sum_sinr / count * 100.0).round() / 100.0 } else { 0.0 };

            let sum_power: f64 = records.iter().filter_map(|r| r.power).sum();
            let avg_power = if count > 0.0 { (sum_power / count * 100.0).round() / 100.0 } else { 0.0 };

            // 4. Vytvorenie záznamu (Kópia tvojej štruktúry)
            let mut summary = first.clone();
            summary.date = first_date;
            summary.time = first_time;
            summary.utc = first_utc;
            summary.latitude = first_lat;
            summary.longitude = first_lon;
            // ... nastavenie ostatných first_ hodnôt ...
            
            summary.rsrp = Some(avg_rsrp);
            summary.sinr = Some(avg_sinr);
            summary.power = Some(avg_power);
            
            summary
        }

    fn create_dummy(&self, mcc: u16, mnc: u16) -> Self {
                LteRecord {
                    // Kopírujeme zo šablóny (self)
                    date: self.date.clone(),
                    time: self.time.clone(),
                    latitude: self.latitude,
                    longitude: self.longitude,
                    altitude: self.altitude,

                    // Nastavíme cieľové MCC a MNC
                    mcc: Some(mcc),
                    mnc: Some(mnc),

                    // Slabý signál
                    power: Some(-174.0),
                    rsrp: Some(-174.0),
                    sinr: Some(-100.0),

                    // Ostatné vyprázdnime
                    utc: None,
                    speed: None,
                    heading: None,
                    num_sat: None,
                    earfcn: None,
                    frequency: None,
                    pci: None,
                    tac: None,
                    ci: None,
                    enodeb_id: None,
                    cell_id: None,
                    bandwidth: None,
                    sym_per_slot: None,
                    rsrq: None,
                    drift_4g: None,
                    sigma_drift_4g: None,
                    time_of_arrival: None,
                    time_of_arrival_fn: None,
                    lte_m: String::new(),
                    nr_5g: String::new(),
                    enodeb_tx_ports: None,
                    sib2_embms_dss: String::new(),
                    mib_dl_bandwidth: None,
                }
            }
        
    fn apply_custom_filter(records: Vec<Self>, filter_files: Option<Vec<PathBuf>>) -> Result<Vec<Self>, Box<dyn Error>> {
            // Ak nie sú filtre, vrátime pôvodné
            let filter_vec = match filter_files {
                Some(v) => v,
                None => return Ok(records),
            };

            let mut rules_map: HashMap<(u16, u16, u64), Vec<(u16, u16)>> = HashMap::new();
            
            // Regexy pre LTE formát filtrov
            let re_mcc = Regex::new(r#""MCC"\s*=\s*(\d+)"#)?;
            let re_mnc = Regex::new(r#""MNC"\s*=\s*(\d+)"#)?;
            let re_freq = Regex::new(r#""Frequency"\s*=\s*(\d+)"#)?;

            for filter_file in filter_vec {
                let path = &filter_file;
                let content = std::fs::read_to_string(path).map_err(|e| format!("Chyba filter {:?}: {}", path, e))?;
                let parts: Vec<&str> = content.split("OR").collect();

                if parts.is_empty() { continue; }

                // Target (prvá časť)
                let first_part = parts[0];
                let target_mcc = match re_mcc.captures(first_part) { Some(c) => c[1].parse::<u16>()?, None => continue };
                let target_mnc = match re_mnc.captures(first_part) { Some(c) => c[1].parse::<u16>()?, None => continue };

                // Sources (zvyšné časti)
                for part in parts.iter().skip(1) {
                    if let (Some(freq_cap), Some(mcc_cap), Some(mnc_cap)) = (re_freq.captures(part), re_mcc.captures(part), re_mnc.captures(part)) {
                        let freq = freq_cap[1].parse::<u64>()?;
                        let src_mcc = mcc_cap[1].parse::<u16>()?;
                        let src_mnc = mnc_cap[1].parse::<u16>()?;
                        
                        rules_map.entry((src_mcc, src_mnc, freq)).or_default().push((target_mcc, target_mnc));
                    }
                }
            }

            // Aplikácia duplikácie
            let mut final_records = records;
            let mut new_duplicates: Vec<Self> = Vec::new();

            for record in &final_records {
                if let (Some(mcc), Some(mnc), Some(freq)) = (record.mcc, record.mnc, record.frequency) {
                    if let Some(targets) = rules_map.get(&(mcc, mnc, freq)) {
                        for (tmcc, tmnc) in targets {
                            let mut dup = record.clone();
                            dup.mcc = Some(*tmcc);
                            dup.mnc = Some(*tmnc);
                            new_duplicates.push(dup);
                        }
                    }
                }
            }
            final_records.extend(new_duplicates);
            Ok(final_records)
        }


    }

impl RecordValidator for LteRecord {
    fn is_valid(&self) -> bool {
        let rsrp = self.rsrp.unwrap_or(9999.0);
        let speed = self.speed.unwrap_or(0.0);

        self.latitude.is_some() 
        && self.longitude.is_some() 
        && self.mcc.is_some() 
        && self.mnc.is_some() 
        && self.sinr.is_some() 
        && self.rsrp.is_some()
        && rsrp < 40.0
        && speed <= 150.0
        && self.pci.is_some()
    }
}

impl RecordFilter for GsmRecord {
    fn get_mcc(&self) -> Option<u16> { self.mcc }
    fn get_mnc(&self) -> Option<u16> { self.mnc }
    fn get_freq(&self) -> Option<u64> { self.frequency }
    fn get_lat(&self) -> Option<f64> { self.latitude }
    fn get_lon(&self) -> Option<f64> { self.longitude }
    fn get_date(&self) -> String { self.date.clone() }
    fn get_total_power(&self) -> Option<f64> { self.total_power }
    fn get_rsrp(&self) -> Option<f64> { None }
    fn get_sinr(&self) -> Option<f64> { None}
    fn get_5g_nr(&self) -> Option<String> { None }
    fn set_lat(&mut self, val: f64) {self.latitude = Some(val);}
    fn set_lon(&mut self, val: f64) {self.longitude = Some(val);}
    fn set_mcc(&mut self, val: u16) { self.mcc = Some(val); }
    fn set_mnc(&mut self, val: u16) { self.mnc = Some(val); }
    fn get_time(&self) -> String { self.time.clone() }
    fn get_pci(&self) -> Option<u16> {
        Some(u16::MAX)
    }
    fn get_signal_strength(&self) -> f64 {
        self.total_power.unwrap_or(-999.0)
    }

    fn create_summary(records: &[Self]) -> Self {
        if records.is_empty() { return records[0].clone(); }
        let first = &records[0];

        // 1. First non-empty logic
        let first_date = records.iter().find(|r| !r.date.is_empty()).map(|r| r.date.clone()).unwrap_or_default();
        let first_time = records.iter().find(|r| !r.time.is_empty()).map(|r| r.time.clone()).unwrap_or_default();
        // ... ostatné ...

        // 2. Averages
        let count = records.len() as f64;
        let sum_total: f64 = records.iter().filter_map(|r| r.total_power).sum();
        let avg_total = if count > 0.0 { (sum_total / count * 100.0).round() / 100.0 } else { 0.0 };

        let sum_sch: f64 = records.iter().filter_map(|r| r.sch_power).sum();
        let avg_sch = if count > 0.0 { (sum_sch / count * 100.0).round() / 100.0 } else { 0.0 };

        // 3. Construct
        let mut summary = first.clone();
        summary.date = first_date;
        summary.time = first_time;
        summary.total_power = Some(avg_total);
        summary.sch_power = Some(avg_sch);
        // ...

        summary
    }

    fn create_dummy(&self, mcc: u16, mnc: u16) -> Self {
        GsmRecord {
            // Kopírujeme zo šablóny
            date: self.date.clone(),
            time: self.time.clone(),
            latitude: self.latitude,
            longitude: self.longitude,
            altitude: self.altitude,

            // Nastavíme cieľové MCC a MNC
            mcc: Some(mcc),
            mnc: Some(mnc),

            // Slabý signál
            total_power: Some(-174.0),
            sch_power: Some(-174.0),

            // Ostatné vyprázdnime
            utc: None,
            frequency: None,
            speed: None,
            heading: None,
            num_sat: None,
            arfcn: None,
            lac: None,
            ci: None,
            bsic: None,
            c2i: None,
            device_id: None,
        }
    }


}

impl RecordValidator for GsmRecord {
    fn is_valid(&self) -> bool {
        let total_power = self.total_power.unwrap_or(9999.0);
        let sch_power = self.sch_power.unwrap_or(9999.0);
        let speed = self.speed.unwrap_or(0.0);

        // 1. Kontrola: Musíme mať identifikátory siete
        self.latitude.is_some()
        && self.longitude.is_some()
        && self.mcc.is_some()
        && self.mnc.is_some()
        && self.frequency.is_some()
        && self.total_power.is_some()
        && self.sch_power.is_some()
        && total_power < 40.0 
        && sch_power < 40.0
        && speed <= 150.0
    }
}

impl RecordValidator for FiveGRecord {
    fn is_valid(&self) -> bool {

        self.latitude.is_some() 
        && self.longitude.is_some() 
        && self.sss_rsrp.is_some()
        && self.sss_sinr.is_some()
        && self.pci.is_some()
    }
}

impl RecordFilter for FiveGRecord {
    fn get_mcc(&self) -> Option<u16> {
        self.mcc
    }
    fn get_mnc(&self) -> Option<u16> {
        self.mnc
    }
    fn get_freq(&self) -> Option<u64> {
        self.ss_ref
    }
    fn get_lat(&self) -> Option<f64> {
        self.latitude
    }
    fn get_lon(&self) -> Option<f64> {
        self.longitude
    }
    fn get_date(&self) -> String {
        self.date.clone()
    }
    fn get_total_power(&self) -> Option<f64> {
        self.sss_re_power
    }
    fn get_rsrp(&self) -> Option<f64> {
        self.sss_rsrp
    }
    fn get_sinr(&self) -> Option<f64> {
        self.sss_sinr
    }
    fn get_5g_nr(&self) -> Option<String> {
        None
    }
    fn get_time(&self) -> String { self.time.clone() }
    fn set_lat(&mut self, val: f64) {self.latitude = Some(val);}
    fn set_lon(&mut self, val: f64) {self.longitude = Some(val);}
    fn set_mcc(&mut self, val: u16) { self.mcc = Some(val); }
    fn set_mnc(&mut self, val: u16) { self.mnc = Some(val); }
    fn get_signal_strength(&self) -> f64 {
        self.sss_rsrp.unwrap_or(-999.0)
    }

    fn get_pci(&self) -> Option<u16> {
        self.pci
    }

    fn create_summary(records: &[Self]) -> Self {
        if records.is_empty() {
            return records[0].clone();
        }

        let first = &records[0];

        // Nájdenie prvého neprázdneho dátumu a času
        let first_date = records.iter()
            .find(|r| !r.date.is_empty())
            .map(|r| r.date.clone())
            .unwrap_or_else(|| first.date.clone());

        let first_time = records.iter()
            .find(|r| !r.time.is_empty())
            .map(|r| r.time.clone())
            .unwrap_or_else(|| first.time.clone());

        // 3. Vytvorenie sumáru (kópia prvého záznamu)
        let mut summary = first.clone();
        summary.date = first_date;
        summary.time = first_time;

        // HELPER: Funkcia na výpočet priemeru pre konkrétne pole
        // Berie len tie záznamy, kde je Some(hodnota) a delí ich skutočným počtom
        let calculate_avg = |extractor: fn(&Self) -> Option<f64>| -> Option<f64> {
            let (sum, count) = records.iter()
                .filter_map(extractor)
                .fold((0.0, 0.0), |(acc_sum, acc_count), val| (acc_sum + val, acc_count + 1.0));

            if count > 0.0 {
                Some((sum / count * 100.0).round() / 100.0)
            } else {
                None // Ak nie sú žiadne hodnoty, vrátime None (nie 0.0, lebo 0.0 je platná hodnota)
            }
        };

        // 4. Správne priemerovanie len pre existujúce hodnoty
        summary.sss_rsrp = calculate_avg(|r| r.sss_rsrp);
        summary.sss_sinr = calculate_avg(|r| r.sss_sinr);
        summary.sss_rsrq = calculate_avg(|r| r.sss_rsrq);
        summary.sss_re_power = calculate_avg(|r| r.sss_re_power);
        summary
    }

    fn create_dummy(&self, mcc: u16, mnc: u16) -> Self {
        FiveGRecord {
            // 1. Kopírujeme údaje zo šablóny (self)
            date: self.date.clone(),
            time: self.time.clone(),
            latitude: self.latitude,
            longitude: self.longitude,
            altitude: self.altitude,

            // 2. Nastavíme cieľové MCC a MNC
            mcc: Some(mcc),
            mnc: Some(mnc),

            // 3. Slabý signál (Dummy hodnoty pre RSRP a SINR)
            sss_rsrp: Some(-174.0), // Teoretický šum
            sss_sinr: Some(-100.0), // Veľmi zlý pomer signálu a šumu

            // 4. Ostatné vyprázdnime (všetko na None)
            utc: None,
            speed: None,
            heading: None,
            num_sat: None,
            nr_arfcn: None,
            ss_ref: None,
            band: None,
            pci: None,
            ssb_idx: None,
            ssb_idx_mod8: None,
            ssb_rssi: None,
            
            // Ostatné SSS polia (okrem rsrp a sinr vyššie)
            sss_rsrq: None,
            sss_re_power: None,

            lac: None,
            rnc_cell_id_h: None,
            rnc_cell_id_d: None,
            toa_pps: None,
            toa_cir: None,

            // MIB polia
            mib_sfn: None,
            mib_scs_common: None,
            mib_ssb_subcarrier_offset: None,
            mib_dmrs_type_a_pos3: None,
            mib_pdcch_config_sib1: None,
            mib_cell_not_barred: None,
            mib_intra_freq_reselection: None,

            // DM_RS
            dm_rs_sinr: None,
            dm_rs_rsrp: None,
            dm_rs_rsrq: None,
            dm_rs_re_power: None,

            // PBCH
            pbch_sinr: None,
            pbch_rsrp: None,
            pbch_rsrq: None,
            pbch_re_power: None,

            // PSS
            pss_sinr: None,
            pss_rsrp: None,
            pss_rsrq: None,
            pss_re_power: None,

            // SSS_PBCH
            sss_pbch_sinr: None,
            sss_pbch_rsrp: None,
            sss_pbch_rsrq: None,
            sss_pbch_re_power: None,

            // SS_PBCH
            ss_pbch_sinr: None,
            ss_pbch_rsrp: None,
            ss_pbch_rsrq: None,
            ss_pbch_re_power: None,

            // CI
            pss_ci_dtol: None,
            pss_ci_dtoh: None,
            sss_ci_dtol: None,
            sss_ci_dtoh: None,

            device_id: None,
            add_plmns: None,
        }
    }

    fn apply_custom_filter(mut records: Vec<Self>, filter_files: Option<Vec<PathBuf>>) -> Result<Vec<Self>, Box<dyn Error>> {
        let filter_vec = match filter_files {
            Some(v) => v,
            None => return Ok(records),
        };

        struct Rule5G {
            target_mnc: u16,
            freq_ranges: Vec<(u64, u64)>,
        }

        let mut rules: Vec<Rule5G> = Vec::new();
        let re_mcc = Regex::new(r#""MCC"\s*=\s*(\d+)"#)?;
        let re_mnc = Regex::new(r#""MNC"\s*=\s*(\d+)"#)?;
        let re_freq_range = Regex::new(r#""Frequency"\s*=\s*(\d+)-(\d+)"#)?;

        // Načítanie pravidiel
        for filter_file in filter_vec {
            let content = std::fs::read_to_string(&filter_file)
                .map_err(|e| format!("Chyba filter {:?}: {}", filter_file, e))?;
            
            let parts: Vec<&str> = content.split("OR").collect();
            if parts.is_empty() { continue; }

            let header = parts[0];
            // 5G pravidlo zrejme mení len MNC v rámci MCC 231, takto to bolo v tvojom kóde
            if re_mcc.captures(header).is_none() { continue; } 
            
            let target_mnc = match re_mnc.captures(header) {
                Some(c) => c[1].parse::<u16>()?,
                None => continue,
            };

            let mut ranges = Vec::new();
            for part in parts.iter().skip(1) {
                if let Some(caps) = re_freq_range.captures(part) {
                    let start = caps[1].parse::<u64>()?;
                    let end = caps[2].parse::<u64>()?;
                    ranges.push((start, end));
                }
            }

            if !ranges.is_empty() {
                rules.push(Rule5G { target_mnc, freq_ranges: ranges });
            }
        }

        // Aplikácia pravidiel (úprava záznamov in-place)
        for record in records.iter_mut() {
            let current_mcc = record.mcc;
            let freq_val = record.ss_ref; // Pozor: 5G má ss_ref namiesto frequency

            // Kontrola MCC (buď 231 alebo None)
            let is_mcc_valid = match current_mcc {
                Some(231) => true,
                None => true,
                _ => false, 
            };

            if is_mcc_valid {
                if let Some(f) = freq_val {
                    for rule in &rules {
                        if rule.freq_ranges.iter().any(|&(start, end)| f >= start && f <= end) {
                            record.mcc = Some(231);
                            record.mnc = Some(rule.target_mnc);
                            break; // Prvé pravidlo vyhráva
                        }
                    }
                }
            }
        }

        Ok(records)
    }

    
}

impl PointComparation for FiveGRecord {
    fn is_secondary_above_threshold(&self, threshold: f32) -> bool {
        self.sss_sinr.unwrap() > threshold as f64
    }

    fn is_primary_above_threshold_and_above_old(&self, threshold: f32, old_value: f64) -> bool {
        self.sss_rsrp.unwrap() > threshold as f64 && self.sss_rsrp.unwrap() > old_value
    }
}

impl PointComparation for LteRecord {
    fn is_secondary_above_threshold(&self, threshold: f32) -> bool {
        self.sinr.unwrap() > threshold as f64
    }

    fn is_primary_above_threshold_and_above_old(&self, threshold: f32, old_value: f64) -> bool {
        self.rsrp.unwrap() > threshold as f64 && self.rsrp.unwrap() > old_value
    }
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.lat.to_bits() == other.lat.to_bits() && self.lon.to_bits() == other.lon.to_bits()
    }
}

#[derive(Debug, Clone, Copy)]
struct Point {
    lat: f64,
    lon: f64,
}

impl Point {
    fn new(lat: f64, lon: f64) -> Self {
        Self { lat, lon }
    }

    fn distance_to(&self, other: &Point) -> f64 {
        Geodesic.distance(GeoPoint::new(self.lon, self.lat), GeoPoint::new(other.lon, other.lat))
    }
}

impl Eq for Point {}

impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.lat.to_bits().hash(state);
        self.lon.to_bits().hash(state);
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)] 
pub struct GridSquare {
    #[serde(rename = "ID")]
    pub id: u32,
    #[serde(rename = "POKRYTI")]
    pub pokryti: i32,
    #[serde(rename = "POCET_OBYV")]
    pub pocet_obyv: i32,
    #[serde(rename = "KOD_OBEC")]
    pub kod_obec: Option<i32>, 
    #[serde(rename = "KOD_OKRES")]
    pub kod_okres: String, 
    #[serde(rename = "SILNICE")]
    pub silnice: i32,
    #[serde(rename = "ZELEZNICE")]
    pub zeleznice: i32,
    #[serde(rename = "X_UTM")]
    pub x_utm: i32,
    #[serde(rename = "Y_UTM")]
    pub y_utm: i32,
}

pub struct RecordPoint {
    pub date: String,
    pub time: String,
    pub lat: f64,
    pub lon: f64,
    pub values: HashMap<(u16,u16), (f64, f64, f64)>
}

pub struct RecordPointMobile {
    pub date: String,
    pub time: String,
    pub lat: f64,
    pub lon: f64,
    pub values: HashMap<(u16,u16), (f64, f64, f64, i32)>
}

pub trait MobilePathProvider: Clone {
    fn lte_pathbuf(&self) -> PathBuf;
    fn g5_pathbuf(&self) -> PathBuf;
}

pub fn get_grid_map(grid_path: PathBuf) -> Result<(HashMap<(i32, i32), u32>, HashMap<u32, GridSquare>), Box<dyn Error>>{
    let mut grid_map: HashMap<(i32, i32), u32> = HashMap::new();
    let mut grid_map_info: HashMap<u32, GridSquare> = HashMap::new();

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b',') // Predpokladáme, že tento súbor je OK a má čiarky
        .from_path(grid_path)?;

    for result in rdr.deserialize() {
        let record: GridSquare = result?;
        grid_map.insert((record.x_utm, record.y_utm), record.id);
        grid_map_info.insert(record.id, record);
    }
    //println!("Načítaných {} 100m zón do pamäte.", count_loaded);
    //println!("--------------------------------------------------");
    Ok((grid_map, grid_map_info))
}

fn read_data_from_csv<T>(input: PathBuf) -> Result<Vec<T>, Box<dyn Error>>
where T: DeserializeOwned + RecordValidator,
{
    let mut file = File::open(input)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    if buffer.starts_with(b"\xEF\xBB\xBF") {
        buffer.drain(0..3);
    }

    let content = String::from_utf8_lossy(&buffer);
    
    let mut reader = BufReader::new(std::io::Cursor::new(content.as_bytes()));
    let mut header_line = String::new();
    let mut found = false;

    while reader.read_line(&mut header_line)? > 0 {
        // Hľadáme kľúčové slová
        if header_line.contains("Date") && header_line.contains("Time") && header_line.contains("UTC") {
            found = true;
            break;
        }
        header_line.clear();
    }

    let delimiter = if header_line.contains(';') {
        b';'
    } else if header_line.contains('\t') {
        b'\t'
    } else {
        b','
    };

    let chain = std::io::Cursor::new(header_line.into_bytes()).chain(reader);

    // 5. Parsovanie
    let mut csv_rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .trim(csv::Trim::All)
        .flexible(true) // <--- TOTO PRIDAJ! (Povolí riadky s extra stĺpcami)
        .from_reader(chain);

    let mut records = Vec::new();
    let mut skipped_count = 0;
    let mut error_count = 0;
    let mut first_error: Option<csv::Error> = None;

    for result in csv_rdr.deserialize() {
        match result {
            Ok(record) => {
                let rec: T = record;
                if rec.is_valid() {
                    records.push(rec);
                } else {
                    skipped_count += 1;
                }
            },
            Err(e) => {
                error_count += 1;
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }
    Ok(records)
}

fn get_txt_from_file(filter_path: PathBuf) -> Result<Vec<PathBuf>, Box<dyn Error>>{
    let mut filter_files: Vec<PathBuf> = Vec::new();

    let entries = fs::read_dir(filter_path)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // 2. Skontrolujeme, či je to súbor (nie podpriečinok)
        if path.is_file() {
            // 3. Skontrolujeme príponu (extension)
            if let Some(extension) = path.extension() {
                // Porovnáme, či je to "txt"
                if extension == "txt" {
                    // Prevedieme PathBuf na String a uložíme
                    //if let Some(path_str) = path.to_str() {
                    //    filter_files.push(path_str.to_string());
                    //}
                    filter_files.push(path);
                }
            }
        }
    }

    Ok(filter_files)
}

fn map_records_to_grid<T>(
    grid_map: HashMap<(i32, i32), u32>,
    records: Vec<T>,
) -> Result<HashMap<(u32, u16, u16, u64, u16), Vec<T>>, Box<dyn Error>>
where
    T: RecordFilter + Clone,
{
    let mut records_hash: HashMap<(u32, u16, u16, u64, u16), Vec<T>> = HashMap::new();
    
    for mut record in records {

        let longitude = record.get_lon().unwrap();
        let latitude = record.get_lat().unwrap();

        let zone: u8 = 34;//get_zone_from_lon(longitude);
        let zone_letter: char = 'U';

        let mut found_id:Option<u32> = None;

        let (northing, easting, _) = to_utm_wgs84(latitude, longitude, zone);

        let x_snap = (easting / 100.0).round() as i32 * 100;
        let y_snap = (northing / 100.0).round() as i32 * 100;

        if let Some(&square_id) = grid_map.get(&(x_snap, y_snap)) {
            found_id = Some(square_id);
        }

        match found_id {
            Some(sq) => {
                //let zone_letter = lat_to_zone_letter(latitude).unwrap();
                let (lat, lon) = wsg84_utm_to_lat_lon(x_snap as f64, y_snap as f64, zone, zone_letter).unwrap();
                
                record.set_lat(lat);
                record.set_lon(lon);

                let mcc = record.get_mcc().unwrap();
                let mnc = record.get_mnc().unwrap();
                let freq = record.get_freq().unwrap();
                let pci = record.get_pci().unwrap();

                records_hash
                .entry((sq, mcc, mnc, freq, pci)) 
                .or_insert_with(Vec::new)  
                .push(record);
            },
            None => (),
        }
    }

    Ok(records_hash)
}

fn get_best_records<T>(
    records_hash: HashMap<(u32, u16, u16, u64, u16), Vec<T>>
) -> HashMap<(u32, u16, u16), T>
where
    T: RecordFilter + Clone, // T musí vedieť filtrovať a klonovať sa
{
    let mut best_records_map: HashMap<(u32, u16, u16), T> = HashMap::new();

    // Iterujeme cez všetky skupiny (Grid, MCC, MNC, Freq)
    for ((grid_id, mcc, mnc, _freq, _pci), records_vec) in records_hash {
        
        if records_vec.is_empty() { continue; }

        // 1. Vytvoríme súhrnný záznam (Average) pomocou Traitu
        // Toto zavolá buď LTE implementáciu alebo GSM implementáciu
        let summary_record = T::create_summary(&records_vec);

        // 2. Kľúč pre finálnu mapu (už bez frekvencie)
        let key = (grid_id, mcc, mnc);

        // 3. Vložíme do mapy alebo porovnáme s existujúcim
        best_records_map
            .entry(key)
            .and_modify(|current_best| {
                // Porovnáme silu signálu cez Trait
                if summary_record.get_signal_strength() > current_best.get_signal_strength() {
                    // Nový záznam je silnejší (lepšia frekvencia), prepíšeme starý
                    *current_best = summary_record.clone();
                }
            })
            .or_insert(summary_record);
    }

    best_records_map
}

fn generate_missing_records<T>(
    best_records_map: HashMap<(u32, u16, u16), T>
) -> HashMap<(u32, u16, u16), T>
where
    T: RecordFilter + Clone,
{
    let mut templates: HashMap<u32, T> = HashMap::new();
    
    let mut best_records_map = best_records_map;

    let mut target_mccs_mncs: HashMap<u16, HashSet<u16>> = HashMap::from([
        (231, HashSet::from([1, 2, 3, 6])),
    ]);

    // 1. Prvý priechod: Zbieranie šablón a existujúcich kombinácií
    for ((grid_id, mcc, mnc), record) in &best_records_map {
        // Uložíme šablónu pre tento štvorec
        if !templates.contains_key(grid_id) {
            templates.insert(*grid_id, record.clone());
        }

        // Pozbierame unikátne MCC a MNC do zoznamu cieľov
        target_mccs_mncs
            .entry(*mcc)
            .or_default() 
            .insert(*mnc);
    }

    // 2. Druhý priechod: Generovanie chýbajúcich
    for (grid_id, template) in templates {
        
        // Iterujeme cez všetkých operátorov
        for (target_mcc, mncs) in &target_mccs_mncs {
            for target_mnc in mncs {
                
                // Kľúč pre KONTROLU (len 3 časti: Kde + Kto)
                let check_key = (grid_id, *target_mcc, *target_mnc);

                // Ak v našom zozname existujúcich NIE JE táto kombinácia...
                if !best_records_map.contains_key(&check_key) {
                    
                    // ...tak vygenerujeme záznam.
                    // Keďže mapa potrebuje kľúč so 4 časťami, pre PCI použijeme 0
                    let map_key = (grid_id, *target_mcc, *target_mnc);

                    // POZOR: Tu záleží, ako máš definovaný RecordFilter.
                    // Ak si vrátil starú verziu bez PCI, tak takto:
                    // let dummy = template.create_dummy(*target_mcc, *target_mnc);
                    
                    // Ak máš novú verziu s PCI, pošli tam 0:
                    let dummy = template.create_dummy(*target_mcc, *target_mnc);
                    
                    best_records_map.insert(map_key, dummy);
                }
            }
        }
    }

    best_records_map
}

fn get_headers<T>(item: &T) -> Result<Vec<String>, Box<dyn Error>> 
where T: Serialize {
    // Vytvoríme "fiktívny" CSV writer do pamäte (nie do súboru)
    let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
    
    // Zapíšeme jeden záznam. CSV writer automaticky zapíše Header + Dáta
    wtr.serialize(item)?;
    
    // Získame výstup ako String
    let data = String::from_utf8(wtr.into_inner()?)?;
    
    // Prečítame prvý riadok (hlavičku)
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false) // Čítame hneď prvý riadok ako dáta
        .from_reader(data.as_bytes());
        
    if let Some(result) = rdr.records().next() {
        let record = result?;
        // Prevedieme na Vec<String>
        return Ok(record.iter().map(|s| s.to_string()).collect());
    }
    
    Ok(vec![])
}

fn save_records_to_csv<T>(
    file_path: PathBuf, 
    records: &HashMap<(u32, u16, u16), T>, 
    grid_map_info: HashMap<u32, GridSquare>
) -> Result<(), Box<dyn Error>> 
where T: Serialize 
{
    let mut file = std::fs::File::create(file_path)?;
    writeln!(file, "")?; // Váš prázdny riadok na začiatku

    // 1. Nastavíme has_headers(false), pretože hlavičku si zapíšeme sami manuálne
    let mut wtr = csv::WriterBuilder::new()
            .has_headers(false) 
            .delimiter(b';')
            .from_writer(file);

    // 2. Musíme zistiť hlavičky. Skúsime nájsť aspoň jeden platný záznam.
    // Nájdeme prvý záznam, ktorý má aj priradený GridSquare
    let first_valid_entry = records.iter().find_map(|((grid_id, _, _), record)| {
        grid_map_info.get(grid_id).map(|grid| (record, grid))
    });

    if let Some((sample_record, sample_grid)) = first_valid_entry {
        // Získame názvy stĺpcov pre T (napr. GsmRecord)
        let headers_rec = get_headers(sample_record)?;
        // Získame názvy stĺpcov pre GridSquare
        let headers_grid = get_headers(sample_grid)?;

        // Spojíme ich dokopy
        let mut all_headers = headers_rec;
        all_headers.extend(headers_grid);

        // Zapíšeme spojenú hlavičku
        wtr.write_record(&all_headers)?;
    }

    // 3. Iterujeme a zapisujeme dáta
    for ((grid_id, _mnc, _lac), record) in records {
        if let Some(grid) = grid_map_info.get(grid_id) {
            // Trik: Serializujeme N-ticu (Tuple). 
            // CSV knižnica zapíše najprv polia z `record` a hneď za nimi polia z `grid`.
            // Tým dosiahneme "flatten" efekt bez použitia mapy.
            wtr.serialize((record, grid))?;
        }
    }

    wtr.flush()?;
    // println!("Hotovo! Dáta sú uložené.");
    // println!("--------------------------------------------------");
    Ok(())
}

pub fn process_dataset<T>(grid_path: PathBuf, input: PathBuf, output: PathBuf,generate_missing_operators: bool, use_filter: bool, filter_path: PathBuf) -> Result<(), String>
where T: DeserializeOwned + Serialize + RecordFilter + RecordValidator + Clone,
{
    let (grid_map, grid_map_info) = get_grid_map(grid_path)
        .map_err(|e| format!("Chyba pri načítaní mriežky: {}", e))?;

    let records = read_data_from_csv::<T>(input)
    .map_err(|e| format!("Chyba pri čítaní CSV dát: {}", e))?;

    let filter_files = if use_filter {
        Some(get_txt_from_file(filter_path).map_err(|e| format!("Chyba načítania filtrov: {}", e))?)
    } else {
        None
    };

    let filtered_records = T::apply_custom_filter(records, filter_files)
        .map_err(|e| format!("Chyba pri aplikácii filtra: {}", e))?;

    let records_hash = map_records_to_grid(grid_map, filtered_records)
        .map_err(|e| format!("Chyba pri mapovaní na grid: {}", e))?;

    let best_records_map = if generate_missing_operators {
        let temp = get_best_records::<T>(records_hash);
        generate_missing_records(temp)
    } else {
        get_best_records::<T>(records_hash)
    };

    save_records_to_csv(output, &best_records_map,grid_map_info)
    .map_err(|e| format!("Chyba pri ukladaní CSV súboru: {}", e))?;

    Ok(())
}

pub fn create_protocol(protocol_path: PathBuf, gsm_path: PathBuf, lte_path: PathBuf, fiveg_path: Option<PathBuf>, output_path: PathBuf, measured_city: String, total_power: f32, sinr: f32, rsrp: f32, antenna_height: f32, internal_environment: f32) -> Result<(), String> {
    fs::copy(protocol_path, &output_path).map_err(|e| format!("Chyba pri kopírovaní protokolu: {}", e))?;
    
    let gsm_records = read_record_and_grid::<GsmRecord>(gsm_path)
        .map_err(|e| format!("Chyba pri čítaní GSM dát: {}", e))?;

    let lte_records = read_record_and_grid::<LteRecord>(lte_path)
        .map_err(|e| format!("Chyba pri čítaní LTE dát: {}", e))?;

    let (fiveg_records, have_5g) = if let Some(path) = fiveg_path {
        (read_record_and_grid::<FiveGRecord>(path)
            .map_err(|e| format!("Chyba pri čítaní 5G dát: {}", e))?, true)
    } else {
        (Vec::new(), false)
    };

    update_excel_cell(&output_path, CITY_CELL, measured_city.trim()).map_err(|e| format!("Chyba pri aktualizácii Excelu: {}", e))?;
    let date = get_date_from_records(&gsm_records)
        .or_else(|| get_date_from_records(&lte_records))
        .or_else(|| Some(String::from("Neznámy dátum"))).unwrap();

    update_excel_cell(&output_path, DATE_CELL, date.as_str()).map_err(|e| format!("Chyba pri aktualizácii Excelu: {}", e))?;
    update_excel_cell(&output_path, ANTENNA_CELL, format!("{} dB", antenna_height).as_str()).map_err(|e| format!("Chyba pri aktualizácii Excelu: {}", e))?;
    update_excel_cell(&output_path, ENVIRONMENT_CELL, format!("{} dB", internal_environment).as_str()).map_err(|e| format!("Chyba pri aktualizácii Excelu: {}", e))?;

    let gsm_table_pos = get_positions_of_table(&output_path, GSM_TABLE_START_CELL).map_err(|e| format!("Chyba pri získavaní pozície tabuľky GSM: {}", e))?;
    let gsm_table_rows = get_rows_of_table(&output_path, gsm_table_pos).map_err(|e| format!("Chyba pri získavaní riadkov tabuľky GSM: {}", e))?;
    let (gsm_calculated_data, stats) = calculate_protocol_data::<GsmRecord>(&gsm_records, &gsm_table_rows, antenna_height, internal_environment, total_power, rsrp, sinr, false);
    fill_row(&output_path, gsm_calculated_data).map_err(|e| format!("Chyba pri vyplneni tabulky: {}",e))?;
    let start_hight = if let Some(pos) = update_excel_cell_smart(&output_path, ZONES_PEOPLE_CELL, stats.as_str(),gsm_table_pos.1).map_err(|e| format!("Chyba pri aktualizácii Excelu: {}", e))? {pos+1} else {1};

    let lte_table_pos = get_positions_of_table(&output_path, LTE_TABLE_START_CELL).map_err(|e| format!("Chyba pri získavaní pozície tabuľky LTE: {}", e))?;
    let lte_table_rows = get_rows_of_table(&output_path, lte_table_pos).map_err(|e| format!("Chyba pri získavaní riadkov tabuľky LTE: {}", e))?;
    let (lte_calculated_data, stats) = calculate_protocol_data::<LteRecord>(&lte_records, &lte_table_rows, antenna_height, internal_environment, total_power, rsrp, sinr, false);
    fill_row(&output_path, lte_calculated_data).map_err(|e| format!("Chyba pri vyplneni tabulky: {}",e))?;
    let start_hight = if let Some(pos) = update_excel_cell_smart(&output_path, ZONES_PEOPLE_CELL, stats.as_str(),lte_table_pos.1).map_err(|e| format!("Chyba pri aktualizácii Excelu: {}", e))? {pos+1} else {1};

    let nr_5g_table_pos = get_positions_of_table(&output_path, NR_5G_TABLE_START_CELL).map_err(|e| format!("Chyba pri získavaní pozície tabuľky NR 5G: {}", e))?;
    let nr_5g_table_rows = get_rows_of_table(&output_path, nr_5g_table_pos).map_err(|e| format!("Chyba pri získavaní riadkov tabuľky NR 5G: {}", e))?;
    let (nr_5g_calculated_data, stats) = if !have_5g 
            {calculate_protocol_data::<LteRecord>(&lte_records, &nr_5g_table_rows, antenna_height, internal_environment, total_power, rsrp, sinr, true)}
        else {
            calculate_protocol_5g_data(&fiveg_records, &lte_records, &nr_5g_table_rows, antenna_height, internal_environment, rsrp, sinr)
        };
    fill_row(&output_path, nr_5g_calculated_data).map_err(|e| format!("Chyba pri vyplneni tabulky: {}",e))?;
    let start_hight = if let Some(pos) = update_excel_cell_smart(&output_path, ZONES_PEOPLE_CELL, stats.as_str(),nr_5g_table_pos.1).map_err(|e| format!("Chyba pri aktualizácii Excelu: {}", e))? {pos+1} else {1};
    Ok(())  
}

fn read_record_and_grid<T>(file_path: PathBuf) -> Result<Vec<(T, GridSquare)>, Box<dyn Error>> where T: DeserializeOwned + RecordFilter {
    
    // 1. Načítanie súboru (Ošetrenie UTF-8 BOM)
    let mut file = File::open(file_path).map_err(|e| format!("Nedá sa otvoriť súbor: {}", e))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    if buffer.starts_with(b"\xEF\xBB\xBF") {
        buffer.drain(0..3);
    }

    let content = String::from_utf8_lossy(&buffer);

    // 2. Nájdenie hlavičky
    let mut reader = BufReader::new(std::io::Cursor::new(content.as_bytes()));
    let mut header_line = String::new();
    let mut found = false;

    // Hľadáme riadok, kde sa začínajú dáta (podľa GsmRecord stĺpcov)
    while reader.read_line(&mut header_line)? > 0 {
        if header_line.contains("Date") && header_line.contains("Time") && header_line.contains("UTC") {
            found = true;
            break;
        }
        header_line.clear();
    }

    if !found {
        return Err("Hlavička CSV nebola nájdená.".into());
    }

    // 3. Detekcia oddeľovača
    let delimiter = if header_line.contains(';') { b';' } 
                    else if header_line.contains('\t') { b'\t' } 
                    else { b',' };

    // Vytvorenie reťazca: hlavička + zvyšok súboru
    let chain = std::io::Cursor::new(header_line.into_bytes()).chain(reader);

    let mut csv_rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .trim(csv::Trim::All)
        .flexible(true) // Dôležité, ak by niektoré riadky mali chybný počet stĺpcov
        .from_reader(chain);

    // 4. Klonovanie hlavičky pre manuálne parsovanie
    let headers = csv_rdr.headers()?.clone();

    let mut data_pairs = Vec::new();
    let mut skipped_count = 0;
    let mut error_count = 0;

    // 5. Hlavný cyklus
    for result in csv_rdr.records() {
        match result {
            Ok(record) => {
                // Pokus o deserializáciu dát z riadku
                let record_res: Result<T, _> = record.deserialize(Some(&headers));
                
                // Pokus o deserializáciu Grid dát z TOHO ISTÉHO riadku
                let grid_res: Result<GridSquare, _> = record.deserialize(Some(&headers));

                match (record_res, grid_res) {
                    (Ok(record), Ok(grid)) => {
                        data_pairs.push((record, grid));
                    },
                    (Err(_), _) | (_, Err(_)) => {
                        // Ak sa nepodarí načítať buď GSM alebo Grid časť (napr. chýbajúce číslo v povinnom poli)
                        error_count += 1;
                    }
                }
            },
            Err(_) => error_count += 1,
        }
    }

    //println!("Štatistika:");
    //println!("  Načítané páry: {}", data_pairs.len());
    //println!("  Chyby formátu: {}", error_count);

    Ok(data_pairs)
}

fn get_date_from_records<T>(records: &Vec<(T, GridSquare)>) -> Option<String> where T: RecordFilter {
    for (record, _) in records {
        if !record.get_date().is_empty() {
            return Some(record.get_date());
        }
    }
    None
}

fn calculate_protocol_data<T>(records: &Vec<(T, GridSquare)>, table_rows: &HashMap<(u32,u32), (u16, u16)>,
        correction_height_value: f32, correction_surauding_value: f32, total_power_value: f32, rsrp_value: f32, sinr_value: f32, nr_5g: bool) 
        -> (HashMap<(u32,u32), Vec<(f32,i32)>>,String) where T: RecordFilter {
    let mut result = HashMap::new();

    let is_lte_mode = records.first()
    .and_then(|(record, _)| record.get_rsrp())
    .is_some();

    // 2. Vyberieme správnu základnú hodnotu
    let base_value = if is_lte_mode { rsrp_value } else { total_power_value };

    // 3. Vypočítame limity zo správnej základnej hodnoty
    let limit_outside = base_value + correction_height_value;
    let limit_inside = base_value + correction_height_value + correction_surauding_value;

    let mut all_zones_count = 0;
    let mut all_people_count: i32 = 0;

    for (pos, (mcc, mnc)) in table_rows {
        let all_records = records.iter()
            .filter(|(record, _)| record.get_mcc() == Some(*mcc) && record.get_mnc() == Some(*mnc))
            .collect::<Vec<&(T, GridSquare)>>();
        let all_count = all_records.len() as f32;
        all_zones_count = all_count as i32;
        let all_count_people: i32 = all_records.iter()
            .map(|(_, square)| square.pocet_obyv)
            .sum();
        all_people_count = all_count_people;

        let valid_records_outside = if is_lte_mode && nr_5g {
            all_records.iter()
            .filter(|(record, _)| {
                record.get_rsrp().unwrap() as f32 >= limit_outside
                    && record.get_sinr().unwrap() as f32 >= sinr_value
                    && has_5g_nr_yes(record.get_5g_nr().as_deref())
            })
            .collect::<Vec<&&(T, GridSquare)>>()
        } else if is_lte_mode {
            all_records.iter()
            .filter(|(record, _)| record.get_rsrp().unwrap() as f32 >= limit_outside && record.get_sinr().unwrap() as f32 >= sinr_value)
            .collect::<Vec<&&(T, GridSquare)>>()
        } else {
            all_records.iter()
            .filter(|(record, _)| limit_outside <= record.get_total_power().unwrap() as f32)
            .collect::<Vec<&&(T, GridSquare)>>()
        };

        let valid_count_outside = valid_records_outside.len() as f32;
        let all_count_people_outside: i32 = valid_records_outside.iter()
            .map(|(_, square)| square.pocet_obyv)
            .sum();

        let valid_records_inside = if is_lte_mode && nr_5g {
            all_records.iter()
            .filter(|(record, _)| {
                record.get_rsrp().unwrap() as f32 >= limit_inside
                    && record.get_sinr().unwrap() as f32 >= sinr_value
                    && has_5g_nr_yes(record.get_5g_nr().as_deref())
            })
            .collect::<Vec<&&(T, GridSquare)>>()
        } else if is_lte_mode {
            all_records.iter()
            .filter(|(record, _)| record.get_rsrp().unwrap() as f32 >= limit_inside && record.get_sinr().unwrap() as f32 >= sinr_value)
            .collect::<Vec<&&(T, GridSquare)>>()
        } else {
            all_records.iter()
            .filter(|(record, _)| limit_inside <= record.get_total_power().unwrap() as f32)
            .collect::<Vec<&&(T, GridSquare)>>()
        };

        let valid_count_inside = valid_records_inside.len() as f32;
        let all_count_people_inside: i32 = valid_records_inside.iter()
            .map(|(_, square)| square.pocet_obyv)
            .sum();

        let fill = if all_count > 0.0 {
            vec![
                (valid_count_outside / all_count,
                valid_count_outside as i32), // Žiadne * 100.0
                (valid_count_inside / all_count,  // Žiadne * 100.0
                valid_count_inside as i32),
                (if all_count_people > 0 { 
                    (all_count_people_outside as f32) / (all_count_people as f32) 
                } else { 0.0 },
                all_count_people_outside),
                
                (if all_count_people > 0 { 
                    (all_count_people_inside as f32) / (all_count_people as f32) 
                } else { 0.0 },
                all_count_people_inside)
            ]
        } else {
            vec![
                (0.0, 0), (0.0, 0), (0.0, 0), (0.0, 0)
            ]
        };
        
        result.insert(*pos, fill);
    }

    (result, if records.len() > 0 {format!("{}/{}", all_zones_count, all_people_count)} else {
        String::from("0/0")
    })
}

fn calculate_protocol_5g_data(
    records: &Vec<(FiveGRecord, GridSquare)>,
    lte_records: &Vec<(LteRecord, GridSquare)>,
    table_rows: &HashMap<(u32,u32), (u16, u16)>,
    correction_height_value: f32,
    correction_surauding_value: f32,
    rsrp_value: f32,
    sinr_value: f32,
) -> (HashMap<(u32,u32), Vec<(f32,i32)>>, String)
{
    let mut result = HashMap::new();

    let limit_outside = rsrp_value + correction_height_value;
    let limit_inside = rsrp_value + correction_height_value + correction_surauding_value;

    // Z LTE záznamov zistíme, ktoré štvorce majú priaznivé LTE + has_5g_nr_yes
    // Rozdelené podľa (mcc, mnc), zvlášť pre outside a inside limity
    let mut lte_valid_outside: HashMap<(u16, u16), HashSet<u32>> = HashMap::new();
    let mut lte_valid_inside: HashMap<(u16, u16), HashSet<u32>> = HashMap::new();

    for (record, grid) in lte_records {
        if has_5g_nr_yes(record.get_5g_nr().as_deref()) {
            if let (Some(mcc), Some(mnc)) = (record.get_mcc(), record.get_mnc()) {
                // LTE priaznivé pre outside
                if record.get_rsrp().unwrap_or(-999.0) as f32 >= limit_outside
                    && record.get_sinr().unwrap_or(-999.0) as f32 >= sinr_value
                {
                    lte_valid_outside.entry((mcc, mnc)).or_insert_with(HashSet::new).insert(grid.id);
                }
                // LTE priaznivé pre inside
                if record.get_rsrp().unwrap_or(-999.0) as f32 >= limit_inside
                    && record.get_sinr().unwrap_or(-999.0) as f32 >= sinr_value
                {
                    lte_valid_inside.entry((mcc, mnc)).or_insert_with(HashSet::new).insert(grid.id);
                }
            }
        }
    }

    let mut all_zones_count = 0;
    let mut all_people_count: i32 = 0;

    for (pos, (mcc, mnc)) in table_rows {
        // Všetky 5G záznamy pre operátora (pre celkový počet zón/obyvateľov)
        let all_records = records.iter()
            .filter(|(record, _)| record.get_mcc() == Some(*mcc) && record.get_mnc() == Some(*mnc))
            .collect::<Vec<&(FiveGRecord, GridSquare)>>();
        let all_count = all_records.len() as f32;
        all_zones_count = all_count as i32;
        let all_count_people: i32 = all_records.iter()
            .map(|(_, square)| square.pocet_obyv)
            .sum();

        all_people_count = all_count_people;

        // 5G štvorec je priaznivý len ak:
        // 1. 5G samotné je priaznivé (RSRP >= limit, SINR >= threshold)
        // 2. LTE pre ten istý štvorec je priaznivé a má nr_5g = yes
        // Ak pre 5G štvorec chýba LTE štvorec → automaticky nepriaznivé
        let valid_records_outside = all_records.iter()
            .filter(|(record, grid)| {
                record.get_rsrp().unwrap() as f32 >= limit_outside
                    && record.get_sinr().unwrap() as f32 >= sinr_value
                    && lte_valid_outside.get(&(*mcc, *mnc)).map_or(false, |set| set.contains(&grid.id))
            })
            .collect::<Vec<&&(FiveGRecord, GridSquare)>>();
        
        let valid_count_outside = valid_records_outside.len() as f32;
        let all_count_people_outside: i32 = valid_records_outside.iter()
            .map(|(_, square)| square.pocet_obyv)
            .sum();

        let valid_records_inside = all_records.iter()
            .filter(|(record, grid)| {
                record.get_rsrp().unwrap() as f32 >= limit_inside
                    && record.get_sinr().unwrap() as f32 >= sinr_value
                    && lte_valid_inside.get(&(*mcc, *mnc)).map_or(false, |set| set.contains(&grid.id))
            })
            .collect::<Vec<&&(FiveGRecord, GridSquare)>>();

        let valid_count_inside = valid_records_inside.len() as f32;
        let all_count_people_inside: i32 = valid_records_inside.iter()
            .map(|(_, square)| square.pocet_obyv)
            .sum();

        let fill = if all_count > 0.0 {
            vec![
                (valid_count_outside / all_count,
                valid_count_outside as i32),
                (valid_count_inside / all_count,
                valid_count_inside as i32),
                (if all_count_people > 0 { 
                    (all_count_people_outside as f32) / (all_count_people as f32) 
                } else { 0.0 },
                all_count_people_outside),
                
                (if all_count_people > 0 { 
                    (all_count_people_inside as f32) / (all_count_people as f32) 
                } else { 0.0 },
                all_count_people_inside)
            ]
        } else {
            vec![
                (0.0, 0), (0.0, 0), (0.0, 0), (0.0, 0)
            ]
        };
        
        result.insert(*pos, fill);
    }

    (result, if records.len() > 0 {format!("{}/{}", all_zones_count, all_people_count)} else {
        String::from("0/0")
    })
}

fn calculate_protocol_db_gsm_data(
    records: &Vec<(GsmRecord, GridSquare)>,
    table_rows: &HashMap<(u32,u32), (u16, u16)>,
    correction_height_value: f32,
    correction_environment_value: f32,
    total_power_value: f32,
) -> (HashMap<(u32,u32), Vec<f64>>, String) {
    let mut result = HashMap::new();

    let limit = total_power_value + correction_height_value + correction_environment_value;

    let mut all_zones_count = 0;
    let mut all_people_count: i32 = 0;

    for (pos, (mcc, mnc)) in table_rows {
        let all_records: Vec<&(GsmRecord, GridSquare)> = records.iter()
            .filter(|(record, _)| record.get_mcc() == Some(*mcc) && record.get_mnc() == Some(*mnc))
            .collect();

        let total_zones = all_records.len() as f64;
        all_zones_count = total_zones as i32;

        let total_people: i32 = all_records.iter()
            .map(|(_, square)| square.pocet_obyv)
            .sum();
        all_people_count = total_people;

        let covered_records: Vec<&&(GsmRecord, GridSquare)> = all_records.iter()
            .filter(|(record, _)| {
                record.get_total_power().unwrap_or(-999.0) as f32 >= limit
            })
            .collect();

        let covered_zones = covered_records.len() as f64;
        let uncovered_zones = total_zones - covered_zones;

        let covered_people: i32 = covered_records.iter()
            .map(|(_, square)| square.pocet_obyv)
            .sum();
        let uncovered_people = total_people - covered_people;

        let fill = if total_zones > 0.0 {
            vec![
                uncovered_zones / total_zones,
                covered_zones,
                uncovered_zones,
                if total_people > 0 { uncovered_people as f64 / total_people as f64 } else { 0.0 },
                covered_people as f64,
                uncovered_people as f64,
            ]
        } else {
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        };

        result.insert(*pos, fill);
    }

    (result, if records.len() > 0 { format!("{}/{}", all_zones_count, all_people_count) } else {
        String::from("0/0")
    })
}

fn calculate_protocol_db_lte_data(
    records: &Vec<(LteRecord, GridSquare)>,
    table_rows: &HashMap<(u32,u32), (u16, u16)>,
    correction_height_value: f32,
    correction_environment_value: f32,
    rsrp_value: f32,
    sinr_value: f32,
) -> (HashMap<(u32,u32), Vec<f64>>, String) {
    let mut result = HashMap::new();

    let limit = rsrp_value + correction_height_value + correction_environment_value;

    let mut all_zones_count = 0;
    let mut all_people_count: i32 = 0;

    for (pos, (mcc, mnc)) in table_rows {
        let all_records: Vec<&(LteRecord, GridSquare)> = records.iter()
            .filter(|(record, _)| record.get_mcc() == Some(*mcc) && record.get_mnc() == Some(*mnc))
            .collect();

        let total_zones = all_records.len() as f64;
        all_zones_count = total_zones as i32;

        let total_people: i32 = all_records.iter()
            .map(|(_, square)| square.pocet_obyv)
            .sum();
        all_people_count = total_people;

        let covered_records: Vec<&&(LteRecord, GridSquare)> = all_records.iter()
            .filter(|(record, _)| {
                record.get_rsrp().unwrap_or(-999.0) as f32 >= limit
                    && record.get_sinr().unwrap_or(-999.0) as f32 >= sinr_value
            })
            .collect();

        let covered_zones = covered_records.len() as f64;
        let uncovered_zones = total_zones - covered_zones;

        let covered_people: i32 = covered_records.iter()
            .map(|(_, square)| square.pocet_obyv)
            .sum();
        let uncovered_people = total_people - covered_people;

        let fill = if total_zones > 0.0 {
            vec![
                uncovered_zones / total_zones,
                covered_zones,
                uncovered_zones,
                if total_people > 0 { uncovered_people as f64 / total_people as f64 } else { 0.0 },
                covered_people as f64,
                uncovered_people as f64,
            ]
        } else {
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        };

        result.insert(*pos, fill);
    }

    (result, if records.len() > 0 { format!("{}/{}", all_zones_count, all_people_count) } else {
        String::from("0/0")
    })
}

fn calculate_protocol_db_5g_data(
    fiveg_records: &Vec<(FiveGRecord, GridSquare)>,
    lte_records: &Vec<(LteRecord, GridSquare)>,
    table_rows: &HashMap<(u32,u32), (u16, u16)>,
    correction_height_value: f32,
    correction_environment_value: f32,
    rsrp_value: f32,
    sinr_value: f32,
) -> (HashMap<(u32,u32), Vec<f64>>, String) {
    let mut result = HashMap::new();

    let limit = rsrp_value + correction_height_value + correction_environment_value;

    let mut all_zones_count = 0;
    let mut all_people_count: i32 = 0;

    // Zistíme, ktoré štvorce sú pokryté LTE (pre každého operátora)
    let mut lte_covered_squares: HashMap<(u16, u16), HashSet<u32>> = HashMap::new();
    for (record, grid) in lte_records {
        if let (Some(mcc), Some(mnc)) = (record.get_mcc(), record.get_mnc()) {
            if record.get_rsrp().unwrap_or(-999.0) as f32 >= limit
                && record.get_sinr().unwrap_or(-999.0) as f32 >= sinr_value
                && has_5g_nr_yes(record.get_5g_nr().as_deref())
            {
                lte_covered_squares.entry((mcc, mnc))
                    .or_insert_with(HashSet::new)
                    .insert(grid.id);
            }
        }
    }

    for (pos, (mcc, mnc)) in table_rows {
        let all_records: Vec<&(FiveGRecord, GridSquare)> = fiveg_records.iter()
            .filter(|(record, _)| record.get_mcc() == Some(*mcc) && record.get_mnc() == Some(*mnc))
            .collect();

        let total_zones = all_records.len() as f64;
        all_zones_count = total_zones as i32;

        let total_people: i32 = all_records.iter()
            .map(|(_, square)| square.pocet_obyv)
            .sum();
        all_people_count = total_people;

        // Štvorec je pokrytý 5G len ak 5G aj LTE spĺňajú podmienky
        let covered_records: Vec<&&(FiveGRecord, GridSquare)> = all_records.iter()
            .filter(|(record, grid)| {
                record.get_rsrp().unwrap_or(-999.0) as f32 >= limit
                    && record.get_sinr().unwrap_or(-999.0) as f32 >= sinr_value
                    && lte_covered_squares.get(&(*mcc, *mnc))
                        .map_or(false, |set| set.contains(&grid.id))
            })
            .collect();

        let covered_zones = covered_records.len() as f64;
        let uncovered_zones = total_zones - covered_zones;

        let covered_people: i32 = covered_records.iter()
            .map(|(_, square)| square.pocet_obyv)
            .sum();
        let uncovered_people = total_people - covered_people;

        let fill = if total_zones > 0.0 {
            vec![
                uncovered_zones / total_zones,
                covered_zones,
                uncovered_zones,
                if total_people > 0 { uncovered_people as f64 / total_people as f64 } else { 0.0 },
                covered_people as f64,
                uncovered_people as f64,
            ]
        } else {
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        };

        result.insert(*pos, fill);
    }

    (result, if fiveg_records.len() > 0 { format!("{}/{}", all_zones_count, all_people_count) } else {
        String::from("0/0")
    })
}

pub fn create_protocol_db(
    protocol_path: PathBuf,
    gsm_path: PathBuf,
    lte_path: PathBuf,
    fiveg_path: PathBuf,
    output_path: PathBuf,
    measured_city: String,
    total_power: f32,
    sinr: f32,
    rsrp: f32,
    antenna_height: f32,
    internal_environment: f32,
) -> Result<(), String> {
    fs::copy(protocol_path, &output_path)
        .map_err(|e| format!("Chyba pri kopírovaní protokolu: {}", e))?;

    let gsm_records = read_record_and_grid::<GsmRecord>(gsm_path)
        .map_err(|e| format!("Chyba pri čítaní GSM dát: {}", e))?;

    let lte_records = read_record_and_grid::<LteRecord>(lte_path)
        .map_err(|e| format!("Chyba pri čítaní LTE dát: {}", e))?;

    let fiveg_records = read_record_and_grid::<FiveGRecord>(fiveg_path)
        .map_err(|e| format!("Chyba pri čítaní 5G dát: {}", e))?;

    // Metadata
    update_excel_cell(&output_path, CITY_CELL, measured_city.trim())
        .map_err(|e| format!("Chyba pri aktualizácii Excelu: {}", e))?;

    let date = get_date_from_records(&gsm_records)
        .or_else(|| get_date_from_records(&lte_records))
        .or_else(|| get_date_from_records(&fiveg_records))
        .or_else(|| Some(String::from("Neznámy dátum"))).unwrap();
    update_excel_cell(&output_path, DATE_CELL, date.as_str())
        .map_err(|e| format!("Chyba pri aktualizácii Excelu: {}", e))?;

    update_excel_cell(&output_path, ANTENNA_CELL, format!("{} dB", antenna_height).as_str())
        .map_err(|e| format!("Chyba pri aktualizácii Excelu: {}", e))?;
    update_excel_cell(&output_path, ENVIRONMENT_CELL, format!("{} dB", internal_environment).as_str())
        .map_err(|e| format!("Chyba pri aktualizácii Excelu: {}", e))?;

    // LTE tabuľka
    let lte_table_pos = get_positions_of_table(&output_path, LTE_TABLE_START_CELL)
        .map_err(|e| format!("Chyba pri získavaní pozície tabuľky LTE: {}", e))?;
    let lte_table_rows = get_rows_of_table(&output_path, lte_table_pos)
        .map_err(|e| format!("Chyba pri získavaní riadkov tabuľky LTE: {}", e))?;
    let (lte_data, stats) = calculate_protocol_db_lte_data(
        &lte_records, &lte_table_rows, antenna_height, internal_environment, rsrp, sinr);
    fill_row_db(&output_path, lte_data)
        .map_err(|e| format!("Chyba pri vyplneni tabulky: {}", e))?;
    let _ = update_excel_cell_smart(&output_path, ZONES_PEOPLE_CELL, stats.as_str(), lte_table_pos.1)
        .map_err(|e| format!("Chyba pri aktualizácii Excelu: {}", e))?;

    // 5G NR tabuľka
    let nr_5g_table_pos = get_positions_of_table(&output_path, NR_5G_TABLE_START_CELL)
        .map_err(|e| format!("Chyba pri získavaní pozície tabuľky NR 5G: {}", e))?;
    let nr_5g_table_rows = get_rows_of_table(&output_path, nr_5g_table_pos)
        .map_err(|e| format!("Chyba pri získavaní riadkov tabuľky NR 5G: {}", e))?;
    let (nr_5g_data, stats) = calculate_protocol_db_5g_data(
        &fiveg_records, &lte_records, &nr_5g_table_rows, antenna_height, internal_environment, rsrp, sinr);
    fill_row_db(&output_path, nr_5g_data)
        .map_err(|e| format!("Chyba pri vyplneni tabulky: {}", e))?;
    let _ = update_excel_cell_smart(&output_path, ZONES_PEOPLE_CELL, stats.as_str(), nr_5g_table_pos.1)
        .map_err(|e| format!("Chyba pri aktualizácii Excelu: {}", e))?;

    // GSM tabuľka
    let gsm_table_pos = get_positions_of_table(&output_path, GSM_TABLE_START_CELL)
        .map_err(|e| format!("Chyba pri získavaní pozície tabuľky GSM: {}", e))?;
    let gsm_table_rows = get_rows_of_table(&output_path, gsm_table_pos)
        .map_err(|e| format!("Chyba pri získavaní riadkov tabuľky GSM: {}", e))?;
    let (gsm_data, stats) = calculate_protocol_db_gsm_data(
        &gsm_records, &gsm_table_rows, antenna_height, internal_environment, total_power);
    fill_row_db(&output_path, gsm_data)
        .map_err(|e| format!("Chyba pri vyplneni tabulky: {}", e))?;
    let _ = update_excel_cell_smart(&output_path, ZONES_PEOPLE_CELL, stats.as_str(), gsm_table_pos.1)
        .map_err(|e| format!("Chyba pri aktualizácii Excelu: {}", e))?;

    Ok(())
}

pub fn process_point_dataset<T>(
    input: Vec<PathBuf>, 
    output: PathBuf,
    filter_path: PathBuf,
    protocol_path: Option<PathBuf>,
    second_output: PathBuf,
    use_filter: bool, 
    max_distance: f64, 
    generate_missing_operators: bool, 
    threshold_sinr: f32, 
    threshold_rsrp: f32
) -> Result<(), String> 
where T: DeserializeOwned + Serialize + RecordFilter + RecordValidator + Clone + PointComparation
{
    let (protocol_path, create_protocol) = if let Some(path) = protocol_path {
        (path, true)
    } else {
        (PathBuf::new(), false)
    }; 

    // 1. Načítame filtre len raz (ak sú zapnuté), aby sme to nerobili v cykle
    let filter_files = if use_filter {
        Some(get_txt_from_file(filter_path).map_err(|e| format!("Chyba načítania filtrov: {}", e))?)
    } else {
        None
    };

    // Sem budeme zbierať finálne spracované výsledky zo všetkých súborov
    // Kľúč je (Bod, MCC, MNC) -> T
    let mut all_final_records: HashMap<(Point, u16, u16), T> = HashMap::new();

    // 2. Iterujeme cez každý vstupný súbor (každý súbor = 1 bod)
    for path in input {
        // A. Načítame dáta len z tohto jedného súboru
        let raw_records = match read_data_from_csv::<T>(path.clone()) {
            Ok(records) => records,
            Err(e) => return Err(format!("Chyba pri čítaní CSV {:?}: {}", path, e)),
        };

        if raw_records.is_empty() {
            continue;
        }

        // B. Aplikujeme filter na tento súbor
        // Poznámka: filter_files musíme klonovať alebo odovzdávať referenciou, 
        // záleží ako máš T::apply_custom_filter definované (tu predpokladám clone pre jednoduchosť, 
        // ideálne by funkcia mala brať &Option<...>)
        let filtered_records = T::apply_custom_filter(raw_records, filter_files.clone())
            .map_err(|e| format!("Chyba pri aplikácii filtra pre {:?}: {}", path, e))?;

        // C. Tu sa stane mágia: assign_records_point vypočíta CENTROID len pre tento jeden súbor
        let hesh_records = assign_records_point::<T>(filtered_records, max_distance)
            .map_err(|e| format!("Chyba pri mapovaní na grid pre {:?}: {}", path, e))?;

        // D. Nájdeme najlepšie signály pre tento bod
        let best_records = if generate_missing_operators {
            let temp = get_best_point_records::<T>(hesh_records, threshold_sinr, threshold_rsrp);
            generate_missing_point_records::<T>(temp)
        } else {
            get_best_point_records::<T>(hesh_records, threshold_sinr, threshold_rsrp)
        };

        // E. Zarovnáme súradnice (aby záznam mal súradnice bodu/centroidu)
        let aligned_records = align_records_to_points::<T>(best_records);

        // F. Pridáme výsledky z tohto súboru do celkovej mapy
        // Ak by sa náhodou stalo, že dva súbory majú úplne identický centroid a operátora,
        // tento `extend` prepíše ten starší (čo je asi ok).
        all_final_records.extend(aligned_records);
    }

    // 3. Uložíme všetko naraz do jedného výstupného CSV
    save_points_to_csv::<T>(output, &all_final_records)
        .map_err(|e| format!("Chyba pri ukladaní CSV súboru: {}", e))?;
    
    if !create_protocol { return Ok(());}

    if second_output.as_os_str().is_empty() {
        return Err("Je zapnuté generovanie 5G/LTE protokolu, ale nezadali ste 'Output Protokol Path' (cieľovú cestu pre Excel).".to_string());
    }

    fs::copy(protocol_path, &second_output).map_err(|e| format!("Chyba pri kopírovaní protokolu: {}", e))?;

    let mut record_points_vec = get_records_vec(&all_final_records);

    write_measurements_to_excel(&second_output, &record_points_vec)
        .map_err(|e| format!("Chyba pri zápise do Excelu: {}", e))?;

    Ok(())
}

pub fn process_multiple_points_dataset<P>(
    input: Vec<P>,
    output: PathBuf,
    lte_filter_path: PathBuf,
    g5_filter_path: PathBuf,
    protocol_path: Option<PathBuf>,
    second_output: PathBuf,
    use_filter: bool,
    max_distance: f64,
    _generate_missing_operators: bool,
    threshold_sinr: f32,
    threshold_rsrp: f32,
) -> Result<(), String>
where
    P: MobilePathProvider,
{
    let (protocol_path, create_protocol) = if let Some(path) = protocol_path {
        (path, true)
    } else {
        (PathBuf::new(), false)
    };

    let lte_filter_files = if use_filter {
        Some(get_txt_from_file(lte_filter_path).map_err(|e| format!("Chyba načítania LTE filtrov: {}", e))?)
    } else {
        None
    };

    let g5_filter_files = if use_filter {
        Some(get_txt_from_file(g5_filter_path).map_err(|e| format!("Chyba načítania 5G filtrov: {}", e))?)
    } else {
        None
    };

    let mut all_final_records: HashMap<(Point, u16, u16), FiveGRecord> = HashMap::new();
    let mut all_nr_map: HashMap<(Point, u16, u16), i32> = HashMap::new();

    for mobile_entry in input {
        let lte_path = mobile_entry.lte_pathbuf();
        let g5_path = mobile_entry.g5_pathbuf();

        let lte_raw_records = read_data_from_csv::<LteRecord>(lte_path.clone())
            .map_err(|e| format!("Chyba pri čítaní LTE CSV {:?}: {}", lte_path, e))?;

        if lte_raw_records.is_empty() {
            continue;
        }

        let lte_filtered_records = LteRecord::apply_custom_filter(lte_raw_records, lte_filter_files.clone())
            .map_err(|e| format!("Chyba pri aplikácii LTE filtra pre {:?}: {}", lte_path, e))?;

        let lte_hash_records = assign_records_point::<LteRecord>(lte_filtered_records, max_distance)
            .map_err(|e| format!("Chyba pri mapovaní LTE bodov pre {:?}: {}", lte_path, e))?;

        // Zistíme pre každý (mcc, mnc) či aspoň 1 LTE záznam v bode má 5G nr = yes/true.
        // Kľúč je len (mcc, mnc) – v rámci jedného mobile_entry je to jeden bod.
        // Ak nájdeme "yes" pre daný (mcc, mnc), uložíme a ideme na ďalší operátor (break).
        let mut lte_nr_map: HashMap<(u16, u16), bool> = HashMap::new();
        for (_point, inner_map) in &lte_hash_records {
            for ((mcc, mnc, _freq, _pci), records_vec) in inner_map {
                let op_key = (*mcc, *mnc);
                // Ak už vieme že tento operátor má yes, preskočíme
                if *lte_nr_map.get(&op_key).unwrap_or(&false) {
                    continue;
                }
                for r in records_vec {
                    if has_5g_nr_yes(r.get_5g_nr().as_deref()) {
                        lte_nr_map.insert(op_key, true);
                        break; // Stačí 1 record s yes, ideme ďalej
                    }
                }
                // Ak sme nenašli yes, nastavíme false (ak ešte nie je)
                lte_nr_map.entry(op_key).or_insert(false);
            }
        }

        let g5_raw_records = read_data_from_csv::<FiveGRecord>(g5_path.clone())
            .map_err(|e| format!("Chyba pri čítaní 5G CSV {:?}: {}", g5_path, e))?;

        let g5_filtered_records = FiveGRecord::apply_custom_filter(g5_raw_records, g5_filter_files.clone())
            .map_err(|e| format!("Chyba pri aplikácii 5G filtra pre {:?}: {}", g5_path, e))?;

        let g5_hash_records = assign_records_point::<FiveGRecord>(g5_filtered_records, max_distance)
            .map_err(|e| format!("Chyba pri mapovaní 5G bodov pre {:?}: {}", g5_path, e))?;

        let g5_temp_records = get_best_point_records::<FiveGRecord>(g5_hash_records, threshold_sinr, threshold_rsrp);
        let g5_best_records = generate_missing_point_records::<FiveGRecord>(g5_temp_records);

        let aligned_records = align_records_to_points::<FiveGRecord>(g5_best_records);

        for ((point, mcc, mnc), rec) in aligned_records {
            let has_nr = *lte_nr_map.get(&(mcc, mnc)).unwrap_or(&false);
            let nr_value = if has_nr { 1 } else { 0 };

            all_nr_map.insert((point, mcc, mnc), nr_value);
            all_final_records.insert((point, mcc, mnc), rec);
        }
    }

    save_mobile_points_to_csv(output, &all_final_records, &all_nr_map)
        .map_err(|e| format!("Chyba pri ukladaní Mobile CSV súboru: {}", e))?;

    if !create_protocol {
        return Ok(());
    }

    if second_output.as_os_str().is_empty() {
        return Err("Je zapnuté generovanie Mobile protokolu, ale nezadali ste 'Output Protokol Path'.".to_string());
    }

    fs::copy(protocol_path, &second_output).map_err(|e| format!("Chyba pri kopírovaní protokolu: {}", e))?;

    let record_points_vec = get_mobile_records_vec(&all_final_records, &all_nr_map);

    write_measurements_to_excel_mobile(&second_output, &record_points_vec)
        .map_err(|e| format!("Chyba pri zápise Mobile meraní do Excelu: {}", e))?;

    Ok(())
}

fn assign_records_point<T>(
    records: Vec<T>,
    max_distance: f64,
) -> Result<HashMap<Point, HashMap<(u16, u16, u64, u16), Vec<T>>>, Box<dyn Error>> 
where T: RecordFilter + Clone, {
    
    // 1. KROK: Vypočítame priemerný stred (centroid)
    let mut sum_lat = 0.0;
    let mut sum_lon = 0.0;
    let mut count = 0;

    // Prvý priechod len na získanie priemeru
    for record in &records {
        if let (Some(lat), Some(lon)) = (record.get_lat(), record.get_lon()) {
            sum_lat += lat;
            sum_lon += lon;
            count += 1;
        }
    }

    if count == 0 {
        return Ok(HashMap::new());
    }

    let avg_lat = sum_lat / count as f64;
    let avg_lon = sum_lon / count as f64;
    
    let center_point = Point::new(avg_lat, avg_lon);

    let mut cells_map: HashMap<(u16, u16, u64, u16), Vec<T>> = HashMap::new();

    for record in records {
        if let (Some(lat), Some(lon), Some(mcc), Some(mnc), Some(freq), Some(pci)) = (
            record.get_lat(),
            record.get_lon(),
            record.get_mcc(),
            record.get_mnc(),
            record.get_freq(),
            record.get_pci(),
        ) {
            let current_point = Point::new(lat, lon);

            if center_point.distance_to(&current_point) <= max_distance {
                let cell_key = (mcc, mnc, freq, pci);

                cells_map
                    .entry(cell_key)
                    .or_insert_with(Vec::new)
                    .push(record);
            }
        }
    }

    let mut hash_map: HashMap<Point, HashMap<(u16, u16, u64, u16), Vec<T>>> = HashMap::new();
    
    if !cells_map.is_empty() {
        hash_map.insert(center_point, cells_map);
    }

    Ok(hash_map)
}

fn get_best_point_records<T>(
    records_hash: HashMap<Point, HashMap<(u16, u16, u64, u16), Vec<T>>>, threshold_sinr: f32, threshold_rsrp: f32
) -> HashMap<(Point, u16, u16), T>
where T: RecordFilter + Clone + PointComparation,
{
    let mut best_records_map: HashMap<(Point, u16, u16), T> = HashMap::new();

    for (point, inner_map) in records_hash {
        
        for ((mcc, mnc, _freq, _pci), records_vec) in inner_map {
            let mut found_best = false;
            if records_vec.is_empty() { continue; }

            // A. Vytvoríme súhrnný záznam (Average) pre túto frekvenciu v tomto bode
            let summary_record = T::create_summary(&records_vec);
            
            // B. Kľúč pre finálne porovnanie: Zaujíma nás Bod + Operátor (frekvenciu v kľúči vynecháme)
            let key = (point, mcc, mnc);

            best_records_map
                .entry(key.clone()) 
                .and_modify(|current_best| {
                    if summary_record.is_secondary_above_threshold(threshold_sinr) 
                       && summary_record.is_primary_above_threshold_and_above_old(threshold_rsrp, current_best.get_signal_strength()) 
                    {
                        *current_best = summary_record.clone();
                        found_best = true;
                    }
                })
                .or_insert(summary_record.clone());

            if !found_best {
                best_records_map
                    .entry(key) 
                    .and_modify(|current_best| {
                        if summary_record.get_signal_strength() > current_best.get_signal_strength() {
                            *current_best = summary_record.clone();
                        }
                    })
                    .or_insert(summary_record); 
            }
        }
    }

    best_records_map
}

fn generate_missing_point_records<T>(
    best_records_map: HashMap<(Point, u16, u16), T>
) -> HashMap<(Point, u16, u16), T>
where T: RecordFilter + Clone,
{
    // Mapa šablón: Pre každý Bod si uložíme jeden vzorový záznam
    let mut templates: HashMap<Point, T> = HashMap::new();

    let mut best_records_map = best_records_map;

    // Predvolení operátori
    let mut target_mccs_mncs: HashMap<u16, HashSet<u16>> = HashMap::from([
        (231, HashSet::from([1, 2, 3, 6])),
    ]);

    // 1. Prvý priechod: Zbieranie šablón a existujúcich kombinácií
    for ((point, mcc, mnc), record) in &best_records_map {
        // Uložíme šablónu (ak ešte nemáme)
        if !templates.contains_key(point) {
            templates.insert(*point, record.clone());
        }

        // Pozbierame unikátne MCC a MNC do zoznamu cieľov
        target_mccs_mncs
            .entry(*mcc)
            .or_default() 
            .insert(*mnc);
    }

    // 2. Druhý priechod: Generovanie chýbajúcich
    for (point, template) in templates {
        
        // Iterujeme cez všetkých cieľových operátorov
        for (target_mcc, mncs) in &target_mccs_mncs {
            for target_mnc in mncs {
                
                // Kľúč pre KONTROLU (len Point + MCC + MNC)
                let check_key = (point, *target_mcc, *target_mnc);

                // Ak v našom zozname existujúcich NIE JE táto kombinácia...
                if !best_records_map.contains_key(&check_key) {
                    
                    // ...tak vygenerujeme záznam.
                    // Do mapy musíme vložiť kľúč so 4 hodnotami, pre PCI použijeme 0.
                    let map_key = (point, *target_mcc, *target_mnc);

                    // Vytvoríme dummy dáta (posielame 0 ako pci)
                    let dummy = template.create_dummy(*target_mcc, *target_mnc);
                    
                    best_records_map.insert(map_key, dummy);
                }
            }
        }
    }

    best_records_map
}

fn align_records_to_points<T>(
    mut best_records_map: HashMap<(Point, u16, u16), T>
) -> HashMap<(Point, u16, u16), T>
where T: RecordFilter,
{
    // Iterujeme cez všetky záznamy ako 'mutable' (meniteľné)
    for ((point, _mcc, _mnc), record) in best_records_map.iter_mut() {
        record.set_lat(point.lat);
        record.set_lon(point.lon);
    }

    best_records_map
}

fn save_points_to_csv<T>(
    file_path: PathBuf, 
    records: &HashMap<(Point, u16, u16), T>
) -> Result<(), Box<dyn Error>> 
where T: Serialize 
{
    let mut file = std::fs::File::create(file_path)?;
    
    // Zachováme tvoj formát s prázdnym riadkom na začiatku
    writeln!(file, "")?; 

    let mut wtr = csv::WriterBuilder::new()
            .has_headers(true) // Zapneme automatické generovanie hlavičky zo štruktúry T
            .delimiter(b';')
            .from_writer(file);

    // Iterujeme len cez hodnoty (records). 
    // Kľúče (Point, mcc, mnc) ignorujeme, pretože:
    // 1. Súradnice Pointu sú už zapísané v 'record.latitude' / 'record.longitude' (vďaka funkcii align_records_to_points)
    // 2. MCC a MNC sú tiež súčasťou záznamu T
    for record in records.values() {
        wtr.serialize(record)?;
    }

    wtr.flush()?;
    Ok(())
}

#[derive(Serialize)]
struct MobileNrCsvRow {
    //5g recod
    #[serde(rename = "5G nr")]
    nr_5g: i32,
}

fn save_mobile_points_to_csv(
    file_path: PathBuf,
    records: &HashMap<(Point, u16, u16), FiveGRecord>,
    nr_map: &HashMap<(Point, u16, u16), i32>,
) -> Result<(), Box<dyn Error>> {
    let mut file = std::fs::File::create(file_path)?;
    writeln!(file, "")?;

    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .delimiter(b';')
        .from_writer(file);

    if let Some((key, sample_record)) = records.iter().next() {
        let mut headers = get_headers(sample_record)?;
        headers.push("5G nr".to_string());
        wtr.write_record(&headers)?;

        let first_nr = nr_map.get(key).cloned().unwrap_or(0);
        wtr.serialize((sample_record, MobileNrCsvRow { nr_5g: first_nr }))?;

        for (iter_key, record) in records.iter().skip(1) {
            let nr_value = nr_map.get(iter_key).cloned().unwrap_or(0);
            wtr.serialize((record, MobileNrCsvRow { nr_5g: nr_value }))?;
        }
    }

    wtr.flush()?;
    Ok(())
}

fn get_records_vec<T>(
    records_hash: &HashMap<(Point, u16, u16), T>
) -> Vec<RecordPoint>
where T: RecordFilter + Clone,
{
    let mut record_points_vec: Vec<RecordPoint> = Vec::new();

    // Iterujeme cez všetky spracované záznamy
    for ((_point_key, mcc, mnc), record) in records_hash {
        

        // Získame dáta z generického záznamu T
        let r_lat = record.get_lat().unwrap_or(0.0);
        let r_lon = record.get_lon().unwrap_or(0.0);
        let r_rsrp = record.get_rsrp().unwrap_or(-140.0);
        let r_sinr = record.get_sinr().unwrap_or(-20.0);
        
        // Konverzia Hz na MHz (celé číslo u64)
        let r_freq_mhz = record.get_freq().unwrap_or(0) as f64 / 1_000_000.0;

        // LOGIKA ZLUČOVANIA:
        // Musíme zistiť, či už tento bod (lat, lon) máme vo vektore.
        // Ak áno -> pridáme doňho nového operátora.
        // Ak nie -> vytvoríme nový RecordPoint.
        
        let existing_point = record_points_vec.iter_mut().find(|rp| {
            rp.lat == r_lat && rp.lon == r_lon
        });

        match existing_point {
            Some(point) => {
                // Bod už existuje, len pridáme hodnoty pre daného operátora (MCC, MNC)
                point.values.insert((*mcc, *mnc), (r_rsrp, r_sinr, r_freq_mhz));
            },
            None => {
                // Bod ešte neexistuje, vytvoríme nový
                let mut vals = HashMap::new();
                vals.insert((*mcc, *mnc), (r_rsrp, r_sinr, r_freq_mhz));

                let new_point = RecordPoint {
                    date: record.get_date(),
                    time: record.get_time(), // Vyžaduje úpravu traitu (viď bod 1)
                    lat: r_lat,
                    lon: r_lon,
                    values: vals,
                };
                record_points_vec.push(new_point);
            }
        }
    }

    record_points_vec
}

fn get_mobile_records_vec(
    records_hash: &HashMap<(Point, u16, u16), FiveGRecord>,
    nr_map: &HashMap<(Point, u16, u16), i32>,
) -> Vec<RecordPointMobile> {
    let mut record_points_vec: Vec<RecordPointMobile> = Vec::new();

    for ((point_key, mcc, mnc), record) in records_hash {
        let r_lat = record.get_lat().unwrap_or(point_key.lat);
        let r_lon = record.get_lon().unwrap_or(point_key.lon);
        let r_rsrp = record.get_rsrp().unwrap_or(-140.0);
        let r_sinr = record.get_sinr().unwrap_or(-20.0);
        let r_freq_mhz = record.get_freq().unwrap_or(0) as f64 / 1_000_000.0;
        let nr_flag = nr_map
            .get(&(*point_key, *mcc, *mnc))
            .cloned()
            .unwrap_or(0);

        let existing_point = record_points_vec.iter_mut().find(|rp| rp.lat == r_lat && rp.lon == r_lon);

        match existing_point {
            Some(point) => {
                point.values.insert((*mcc, *mnc), (r_rsrp, r_sinr, r_freq_mhz, nr_flag));
            }
            None => {
                let mut vals = HashMap::new();
                vals.insert((*mcc, *mnc), (r_rsrp, r_sinr, r_freq_mhz, nr_flag));

                record_points_vec.push(RecordPointMobile {
                    date: record.get_date(),
                    time: record.get_time(),
                    lat: r_lat,
                    lon: r_lon,
                    values: vals,
                });
            }
        }
    }

    record_points_vec
}