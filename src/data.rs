use std::path::Path;

use crate::column::{PlotData, RawColumn};
use crate::error::GglangError;

pub fn load_csv(path: &Path) -> Result<PlotData, GglangError> {
    let content = std::fs::read_to_string(path).map_err(|e| GglangError::Data {
        message: format!("Failed to read CSV: {}", e),
    })?;

    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| GglangError::Data {
        message: "CSV file is empty".to_string(),
    })?;
    let columns: Vec<&str> = header.split(',').map(|s| s.trim()).collect();

    // Read all values as strings first
    let mut string_data: Vec<Vec<String>> = vec![vec![]; columns.len()];

    for (line_num, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let values: Vec<&str> = line.split(',').collect();
        if values.len() != columns.len() {
            return Err(GglangError::Data {
                message: format!(
                    "Row {} has {} columns, expected {}",
                    line_num + 2,
                    values.len(),
                    columns.len()
                ),
            });
        }
        for (i, val) in values.iter().enumerate() {
            string_data[i].push(val.trim().to_string());
        }
    }

    // Per column: if all values parse as f64, use FloatArray; otherwise StringArray
    let mut plot_data = PlotData::new();
    for (i, col) in columns.iter().enumerate() {
        let floats: Result<Vec<f64>, _> = string_data[i].iter().map(|s| s.parse::<f64>()).collect();
        let param = match floats {
            Ok(v) => RawColumn::FloatArray(v),
            Err(_) => RawColumn::StringArray(string_data[i].clone()),
        };
        plot_data.insert(col.to_string(), param);
    }

    Ok(plot_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_load_csv_mixed_types() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_mixed.csv");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "x,y,species").unwrap();
        writeln!(f, "1,2,setosa").unwrap();
        writeln!(f, "3,4,versicolor").unwrap();
        writeln!(f, "5,6,setosa").unwrap();

        let data = load_csv(&path).expect("Should load CSV");
        assert!(data.contains("x"));
        assert!(data.contains("species"));

        // Numeric columns should be FloatArray
        match data.get("x").unwrap() {
            RawColumn::FloatArray(v) => assert_eq!(v, &[1.0, 3.0, 5.0]),
            other => panic!("Expected FloatArray for x, got {:?}", std::mem::discriminant(other)),
        }

        // String columns should be StringArray
        match data.get("species").unwrap() {
            RawColumn::StringArray(v) => {
                assert_eq!(v, &["setosa", "versicolor", "setosa"]);
            }
            other => panic!("Expected StringArray for species, got {:?}", std::mem::discriminant(other)),
        }
    }

    #[test]
    fn test_load_csv() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_scatter.csv");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "x,y").unwrap();
        writeln!(f, "1.0,2.0").unwrap();
        writeln!(f, "3.0,4.0").unwrap();

        let data = load_csv(&path).expect("Should load CSV");
        assert!(data.contains("x"));
        assert!(data.contains("y"));
    }
}
