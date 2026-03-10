use std::{collections::HashMap, path::{Path, PathBuf}};
use umya_spreadsheet;
use calamine::{Reader, Xlsx, open_workbook,Data};
use crate::managers::data_manager::{RecordPoint, RecordPointMobile};

// --- Pomocné funkcie (ponechaj ich v súbore nad update_excel_cell) ---

fn column_name_to_number(name: &str) -> u32 {
    let mut result: u32 = 0;
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            result = result * 26 + (c as u32 - 'A' as u32 + 1);
        }
    }
    result
}

fn parse_coordinate(coord: &str) -> (u32, u32) {
    let alpha_part: String = coord.chars().take_while(|c| c.is_alphabetic()).collect();
    let num_part: String = coord.chars().skip_while(|c| c.is_alphabetic()).collect();

    let col = column_name_to_number(&alpha_part);
    let row = num_part.parse::<u32>().unwrap_or(0);
    (col, row)
}

fn parse_range(range: &str) -> (u32, u32, u32, u32) {
    let parts: Vec<&str> = range.split(':').collect();
    if parts.len() == 2 {
        let (s_col, s_row) = parse_coordinate(parts[0]);
        let (e_col, e_row) = parse_coordinate(parts[1]);
        (s_col, s_row, e_col, e_row)
    } else {
        let (c, r) = parse_coordinate(parts[0]);
        (c, r, c, r)
    }
}

pub fn update_excel_cell(file_path: &PathBuf, search_text: &str, new_value: &str) -> Result<(), Box<dyn std::error::Error>> {

    let mut book = umya_spreadsheet::reader::xlsx::read(file_path)
        .map_err(|e| format!("Chyba pri čítaní Excelu: {}", e))?;

    let sheet = book.get_sheet_mut(&0)
        .ok_or("Súbor neobsahuje žiadne hárky")?;

    let mut found_col: u32 = 0;
    let mut found_row: u32 = 0;
    let mut found = false;

    // Hľadanie bunky
    for cell in sheet.get_cell_collection() {
        let val = cell.get_value();
        if val.trim().contains(search_text.trim()) {
            found_col = *cell.get_coordinate().get_col_num();
            found_row = *cell.get_coordinate().get_row_num();
            found = true;
            break; 
        }
    }

    if found {
        // Default: píšeme do nasledujúceho stĺpca
        let mut target_col = found_col + 1;

        // --- OPRAVENÁ ČASŤ PRE MERGE CELLS ---
        // get_merge_cells() vracia priamo zoznam (slice), nie Option
        let merge_cells = sheet.get_merge_cells();
        
        for range_obj in merge_cells {
            // Z objektu Range získame string (napr. "A1:C1")
            let range_str = range_obj.get_range();
            
            // Rozparsujeme ho našou funkciou
            let (min_col, min_row, max_col, max_row) = parse_range(range_str.as_str());

            // Ak sa naša nájdená bunka nachádza v tomto rozsahu
            if found_col >= min_col && found_col <= max_col &&
               found_row >= min_row && found_row <= max_row {
                
                //println!("Bunka je súčasťou zlúčenia: {}", range_str);
                // Posunieme cieľ až za koniec zlúčenia
                target_col = max_col + 1;
                break;
            }
        }
        sheet.get_cell_mut((target_col, found_row)).set_value(new_value);

        let _ = umya_spreadsheet::writer::xlsx::write(&book, file_path)
            .map_err(|e| format!("Chyba pri ukladaní Excelu: {}", e))?;
    } else {
        println!("Text '{}' sa v súbore nenašiel.", search_text);
    }

    Ok(())
}

pub fn get_positions_of_table(file_path: &PathBuf, table_name: &str) -> Result<(u32,u32), Box<dyn std::error::Error>> {
    let mut workbook: Xlsx<_> = open_workbook(file_path)?;

    if let Some(Ok(range)) = workbook.worksheet_range_at(0) {
        
        for (row_idx, row) in range.rows().enumerate() {
            
            for (col_idx, cell) in row.iter().enumerate() {
                
                let obsah = match cell {
                    Data::Empty => String::new(),
                    val => val.to_string(),
                };

                if obsah.contains(table_name){
                    return Ok((col_idx as u32, row_idx as u32));
                }
            }
        }
    }

    Err(format!("Tabuľka obsahujuca {} sa nenašla.", table_name).into())
}

pub fn get_rows_of_table(file_path: &PathBuf, table_pos: (u32, u32)) -> Result<HashMap<(u32, u32), (u16, u16)>, Box<dyn std::error::Error>> {
    let mut map = HashMap::new();

    let mut inside_table = false;

    let mut workbook: Xlsx<_> = open_workbook(file_path)?;

    if let Some(Ok(range)) = workbook.worksheet_range_at(0) {
        
        let target_col = table_pos.0 as u32;
        let start_row = table_pos.1 as u32;

        for row_idx in start_row..range.height() as u32 {
            let cell = range.get_value((row_idx, target_col));
            
            let raw_text = cell.map(|c| c.to_string()).unwrap_or_default();

            let obsah: Vec<&str> = raw_text.trim().split(' ').collect();

            if obsah.get(0).unwrap_or(&"None").parse::<u16>().is_ok() && obsah.get(1).unwrap_or(&"None").parse::<u16>().is_ok() {
                inside_table = true;
                map.insert((target_col, row_idx), (obsah.get(0).unwrap().parse::<u16>()?, obsah.get(1).unwrap().parse::<u16>()?));
            } else {
                if inside_table {
                    break;
                }
            }
        }
    }

    Ok(map)
}

pub fn fill_row(file_path: &PathBuf, rows_info: HashMap<(u32,u32), Vec<(f32,i32)>>) -> Result<(), Box<dyn std::error::Error>>{

    let mut book = umya_spreadsheet::reader::xlsx::read(file_path)
        .map_err(|e| format!("Chyba pri čítaní Excelu: {}", e))?;

    let sheet = book.get_sheet_mut(&0)
        .ok_or("Súbor neobsahuje žiadne hárky")?;

    for ((start_col, row), data_vec) in rows_info {
        
        // Oprava indexovania (Calamine 0-based -> Umya 1-based)
        let mut current_col = start_col + 1; 
        let target_row = row + 1; 

        for value in data_vec.iter() {
            current_col += 1; 
            
            // --- KĽÚČOVÁ ZMENA ---
            // Namiesto set_value(string) použijeme set_value_number(float)
            // Tým sa odstráni "Number stored as text" a Excel použije svoje formátovanie.
            sheet.get_cell_mut((current_col, target_row))
                 .set_value_number(value.0); 
            
            current_col += 1; 
            sheet.get_cell_mut((current_col, target_row))
                 .set_value_number(value.1); 
        }
    }

    // Nezabudni na uloženie!
    let _ = umya_spreadsheet::writer::xlsx::write(&book, file_path)
        .map_err(|e| format!("Chyba pri ukladaní Excelu: {}", e))?;

    Ok(())
}

pub fn update_excel_cell_smart(
    file_path: &PathBuf,
    search_text: &str,
    new_value: &str,
    start_from_row: u32,
) -> Result<Option<u32>, Box<dyn std::error::Error>> {

    let mut book = umya_spreadsheet::reader::xlsx::read(file_path)
        .map_err(|e| format!("Chyba pri čítaní Excelu: {}", e))?;

    let sheet = book.get_sheet_mut(&0)
        .ok_or("Súbor neobsahuje žiadne hárky")?;

    let clean_search = search_text.replace(|c: char| c.is_whitespace(), "").to_lowercase();
    
    // Zozbierame VŠETKY výskyty
    let mut candidates: Vec<(u32, u32)> = Vec::new();

    for cell in sheet.get_cell_collection() {
        let current_row = *cell.get_coordinate().get_row_num();

        // Ignorujeme všetko, čo je nad našou tabuľkou
        if current_row < start_from_row {
            continue;
        }

        let val = cell.get_value();
        if val.is_empty() { continue; }

        let clean_val = val.replace(|c: char| c.is_whitespace(), "").to_lowercase();

        if clean_val.contains(&clean_search) {
            candidates.push((current_row, *cell.get_coordinate().get_col_num()));
        }
    }

    // ZORADÍME ich podľa riadku (od najmenšieho po najväčší)
    // Toto je kľúčové! Zaistí to, že nájdeme ten výskyt, ktorý patrí k našej tabuľke,
    // a nie nejaký ďalší v poradí.
    candidates.sort_by_key(|k| k.0);

    if let Some((found_row, found_col)) = candidates.first() {
        let found_row = *found_row;
        let found_col = *found_col;

        let mut target_col = found_col + 1;

        // Kontrola Merge Cells
        let merge_cells = sheet.get_merge_cells();
        for range_obj in merge_cells {
            let range_str = range_obj.get_range();
            let (min_col, min_row, max_col, max_row) = parse_range(range_str.as_str());

            if found_col >= min_col && found_col <= max_col &&
               found_row >= min_row && found_row <= max_row {
                target_col = max_col + 1;
                break;
            }
        }

        sheet.get_cell_mut((target_col, found_row)).set_value(new_value);

        let _ = umya_spreadsheet::writer::xlsx::write(&book, file_path)
            .map_err(|e| format!("Chyba pri ukladaní Excelu: {}", e))?;
        
        return Ok(Some(found_row));
    } else {
        println!("Text '{}' sa nenašiel od riadku {}.", search_text, start_from_row);
        return Ok(None);
    }
}

pub fn write_measurements_to_excel(
    file_path: &PathBuf,
    data: &Vec<RecordPoint>
) -> Result<(), Box<dyn std::error::Error>> {

    // 1. Načítame Excel
    let mut book = umya_spreadsheet::reader::xlsx::read(file_path)
        .map_err(|e| format!("Chyba pri čítaní Excelu: {}", e))?;

    let sheet = book.get_sheet_mut(&0)
        .ok_or("Súbor neobsahuje žiadne hárky")?;

    // 2. Nájdeme hlavičku "Dátum"
    let mut header_row = 0;
    let mut date_col = 0;
    let mut found = false;

    for cell in sheet.get_cell_collection() {
        // Tu tiež používame .get_value() pre istotu, hoci pri iterácii to zvyčajne funguje
        if cell.get_value().trim() == "Dátum" {
            header_row = *cell.get_coordinate().get_row_num();
            date_col = *cell.get_coordinate().get_col_num();
            found = true;
            break;
        }
    }

    if !found {
        return Err("V Exceli sa nenašla bunka s textom 'Dátum'".into());
    }

    // 3. Zmapujeme stĺpce pre operátorov (MCC, MNC)
    // Hľadáme v riadku NAD hlavičkou (header_row - 1)
    let operator_row = if header_row > 1 { header_row - 1 } else { return Err("Hlavička je príliš vysoko".into()); };
    let mut operator_columns: HashMap<(u16, u16), u32> = HashMap::new();

    let max_col = date_col + 100; // Hľadáme max 100 stĺpcov doprava
    
    // Začneme hľadať od stĺpca za Longitude (Date + 3 stĺpce sú Date, Time, Lat, Lon)
    let mut col = date_col + 4; 
    
    while col < max_col {
        // 1. Získame objekty buniek
        let mcc_cell = sheet.get_cell_value((col, operator_row));
        let mnc_cell = sheet.get_cell_value((col + 1, operator_row));

        // 2. Získame textovú hodnotu a ULOŽÍME JU (aby prežila)
        // Toto vytvorí String, ktorý bude žiť do konca tohto bloku {}
        let mcc_raw_string = mcc_cell.get_value();
        let mnc_raw_string = mnc_cell.get_value();

        // 3. Teraz môžeme bezpečne urobiť trim() (požičiame si dáta z premenných vyššie)
        let mcc_str = mcc_raw_string.trim();
        let mnc_str = mnc_raw_string.trim();

        if !mcc_str.is_empty() && !mnc_str.is_empty() {
            // Skúsime parsovať (najprv ako float kvôli Excelu "231.0", potom u16)
            let mcc_parsed = mcc_str.parse::<f64>().map(|f| f as u16).or_else(|_| mcc_str.parse::<u16>());
            let mnc_parsed = mnc_str.parse::<f64>().map(|f| f as u16).or_else(|_| mnc_str.parse::<u16>());

            if let (Ok(mcc), Ok(mnc)) = (mcc_parsed, mnc_parsed) {
                // Našli sme stĺpec pre tohto operátora
                operator_columns.insert((mcc, mnc), col);
                
                // Posunieme sa o 3 (RSRP, SINR, Freq)
                col += 3; 
                continue;
            }
        }
        col += 1;
    }

    // 4. Zápis dát
    let mut current_row = header_row + 1;

    for point in data {
        // A. Základné údaje
        sheet.get_cell_mut((date_col, current_row)).set_value(&point.date);
        sheet.get_cell_mut((date_col + 1, current_row)).set_value(&point.time);
        sheet.get_cell_mut((date_col + 2, current_row)).set_value_number(point.lat);
        sheet.get_cell_mut((date_col + 3, current_row)).set_value_number(point.lon);

        // B. Hodnoty pre operátorov
        for ((mcc, mnc), (rsrp, sinr, freq)) in &point.values {
            if let Some(&start_col) = operator_columns.get(&(*mcc, *mnc)) {
                // RSRP
                sheet.get_cell_mut((start_col, current_row)).set_value_number(*rsrp);
                // SINR
                sheet.get_cell_mut((start_col + 1, current_row)).set_value_number(*sinr);
                // Frekvencia (v Exceli ako číslo)
                sheet.get_cell_mut((start_col + 2, current_row)).set_value_number(*freq as f64);
            }
        }
        current_row += 1;
    }

    // 5. Uloženie
    umya_spreadsheet::writer::xlsx::write(&book, file_path)
        .map_err(|e| format!("Chyba pri ukladaní Excelu: {}", e))?;

    Ok(())
}

pub fn write_measurements_to_excel_mobile(
    file_path: &PathBuf,
    data: &Vec<RecordPointMobile>
) -> Result<(), Box<dyn std::error::Error>> {
    let mut book = umya_spreadsheet::reader::xlsx::read(file_path)
        .map_err(|e| format!("Chyba pri čítaní Excelu: {}", e))?;

    let sheet = book.get_sheet_mut(&0)
        .ok_or("Súbor neobsahuje žiadne hárky")?;

    let mut header_row = 0;
    let mut date_col = 0;
    let mut found = false;

    for cell in sheet.get_cell_collection() {
        if cell.get_value().trim() == "Dátum" {
            header_row = *cell.get_coordinate().get_row_num();
            date_col = *cell.get_coordinate().get_col_num();
            found = true;
            break;
        }
    }

    if !found {
        return Err("V Exceli sa nenašla bunka s textom 'Dátum'".into());
    }

    let operator_row = if header_row > 1 { header_row - 1 } else { return Err("Hlavička je príliš vysoko".into()); };
    let mut operator_columns: HashMap<(u16, u16), u32> = HashMap::new();

    let max_col = date_col + 200;
    let mut col = date_col + 4;

    while col < max_col {
        let mcc_cell = sheet.get_cell_value((col, operator_row));
        let mnc_cell = sheet.get_cell_value((col + 1, operator_row));

        let mcc_raw_string = mcc_cell.get_value();
        let mnc_raw_string = mnc_cell.get_value();

        let mcc_str = mcc_raw_string.trim();
        let mnc_str = mnc_raw_string.trim();

        if !mcc_str.is_empty() && !mnc_str.is_empty() {
            let mcc_parsed = mcc_str.parse::<f64>().map(|f| f as u16).or_else(|_| mcc_str.parse::<u16>());
            let mnc_parsed = mnc_str.parse::<f64>().map(|f| f as u16).or_else(|_| mnc_str.parse::<u16>());

            if let (Ok(mcc), Ok(mnc)) = (mcc_parsed, mnc_parsed) {
                operator_columns.insert((mcc, mnc), col);
                col += 4;
                continue;
            }
        }
        col += 1;
    }

    let mut current_row = header_row + 1;

    for point in data {
        sheet.get_cell_mut((date_col, current_row)).set_value(&point.date);
        sheet.get_cell_mut((date_col + 1, current_row)).set_value(&point.time);
        sheet.get_cell_mut((date_col + 2, current_row)).set_value_number(point.lat);
        sheet.get_cell_mut((date_col + 3, current_row)).set_value_number(point.lon);

        for ((mcc, mnc), (rsrp, sinr, freq, nr_5g)) in &point.values {
            if let Some(&start_col) = operator_columns.get(&(*mcc, *mnc)) {
                sheet.get_cell_mut((start_col, current_row)).set_value_number(*rsrp);
                sheet.get_cell_mut((start_col + 1, current_row)).set_value_number(*sinr);
                sheet.get_cell_mut((start_col + 2, current_row)).set_value_number(*freq as f64);
                let nr_5g_str = if *nr_5g == 1 { "yes" } else { "no" };
                sheet.get_cell_mut((start_col + 3, current_row)).set_value(nr_5g_str);
            }
        }

        current_row += 1;
    }

    umya_spreadsheet::writer::xlsx::write(&book, file_path)
        .map_err(|e| format!("Chyba pri ukladaní Excelu: {}", e))?;

    Ok(())
}