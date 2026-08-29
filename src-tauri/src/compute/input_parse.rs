//! 输入文件（xlsx）后端解析（FR-1）
//!
//! 前端上传输入文件后，由后端解析「项目 / 数值」参数表并返回结果，
//! 前端将解析值回显到参数表单，确保执行计算的参数与传入文档一致。
//!
//! 模板结构：inputtemplate_ldzl_3.0.xlsx · 含「项目」「数值」表头行，
//! 其后每行一项（与前端 fileParsers.INPUT_LABEL_KEYS 口径一致）。

use std::collections::BTreeMap;
use std::io::Cursor;

use calamine::{Data, Reader, Xlsx};
use serde::Serialize;

/// 输入文件「项目」名称 → 前端参数键（useParams.params，与前端映射保持一致）
const INPUT_LABEL_KEYS: &[(&str, &str)] = &[
    ("储能充放电深度", "dod"),
    ("电池充放电倍率", "rate"),
    ("储能初始电量", "initialSoc"),
    ("储能系统充电效率", "chargeEff"),
    ("储能系统放电效率", "dischargeEff"),
    ("接入公共电网容量（最大下网功率）", "gridCapacity"),
    ("平均负荷率", "avgLoadRate"),
    ("自发自用占总可用发电量比例下限", "selfUseGenMin"),
    ("自发自用占总用电量比例下限", "selfUseLoadMin"),
    ("余电上网比例上限", "feedLimit"),
    ("余电最大上网功率", "feedPower"),
    ("弃电率上限", "curtailLimit"),
    ("风电系统单位投资", "windInvest"),
    ("光伏系统单位投资", "pvInvest"),
    ("储能系统单位投资", "essInvest"),
    ("年运维费用占比", "opexRatio"),
    ("人员工资", "salary"),
    ("定员人数", "staffCount"),
    ("折现率", "discountRate"),
    ("评价周期", "evalPeriod"),
    ("其他固定投资", "otherInvest"),
    ("电池更换单价", "batteryReplaceUnit"),
    ("电池更换比例", "batteryReplaceRatio"),
    ("电池更换时间", "batteryReplaceYear"),
    ("选定风电规模起始值", "windStart"),
    ("选定风电规模结束值", "windEnd"),
    ("选定光伏规模起始值", "pvStart"),
    ("选定光伏规模结束值", "pvEnd"),
    ("选定储能容量起始值", "essStart"),
    ("选定储能容量结束值", "essEnd"),
];

/// 输入文件解析结果（返回给前端用于参数回填）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputParsePayload {
    /// 参数键 → 数值（前端按 useUploadedFiles.applyInputFile 逻辑回填）
    pub values: BTreeMap<String, f64>,
    /// 成功识别并回填的参数名
    pub applied_labels: Vec<String>,
    /// 无法识别的行（不在标准模板内的项目）
    pub skipped_labels: Vec<String>,
    /// 解析使用的工作表名
    pub sheet_name: String,
}

/// 单元格文本（去除首尾空白）
fn cell_text(d: &Data) -> String {
    match d {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_string(),
        other => other.to_string().trim().to_string(),
    }
}

/// 单元格数值（兼容浮点 / 整数 / 千分位文本）
fn cell_number(d: &Data) -> Option<f64> {
    match d {
        Data::Float(f) if f.is_finite() => Some(*f),
        Data::Int(i) => Some(*i as f64),
        Data::String(s) => {
            let t = s.trim().replace(',', "");
            if t.is_empty() {
                return None;
            }
            t.parse::<f64>().ok().filter(|v| v.is_finite())
        }
        _ => None,
    }
}

/// 解析输入文件字节，提取「项目 / 数值」参数。
/// 结构：表头行含「项目」与「数值」列，其后每行一项。
pub fn parse_input_xlsx(bytes: &[u8]) -> Result<InputParsePayload, String> {
    let cursor = Cursor::new(bytes);
    let mut wb: Xlsx<_> = Xlsx::new(cursor).map_err(|e| format!("输入文件读取失败：{e}"))?;

    let sheet_names = wb.sheet_names();
    if sheet_names.is_empty() {
        return Err("输入文件中没有工作表".to_string());
    }
    let sheet_name = sheet_names
        .iter()
        .find(|n| n.to_lowercase().contains("input"))
        .cloned()
        .unwrap_or_else(|| sheet_names[0].clone());
    let range = wb
        .worksheet_range(&sheet_name)
        .map_err(|e| format!("读取工作表「{sheet_name}」失败：{e}"))?;

    let rows: Vec<Vec<Data>> = range.rows().map(<[Data]>::to_vec).collect();

    let header_idx = rows
        .iter()
        .position(|r| {
            let cells: Vec<String> = r.iter().map(cell_text).collect();
            cells.iter().any(|c| c == "项目") && cells.iter().any(|c| c == "数值")
        })
        .ok_or_else(|| {
            format!(
                "工作表「{sheet_name}」中未找到「项目 / 数值」表头，请使用标准输入文件模板"
            )
        })?;

    let header: Vec<String> = rows[header_idx].iter().map(cell_text).collect();
    let label_col = header.iter().position(|c| c == "项目").unwrap_or(0);
    let value_col = header.iter().position(|c| c == "数值").unwrap_or(1);

    let mut values = BTreeMap::new();
    let mut applied_labels: Vec<String> = Vec::new();
    let mut skipped_labels: Vec<String> = Vec::new();

    for row in rows.iter().skip(header_idx + 1) {
        let label = row.get(label_col).map(cell_text).unwrap_or_default();
        if label.is_empty() {
            continue;
        }
        let number = row.get(value_col).and_then(cell_number);
        match (
            INPUT_LABEL_KEYS.iter().find(|(l, _)| *l == label),
            number,
        ) {
            (Some((_, key)), Some(v)) => {
                values.insert((*key).to_string(), v);
                applied_labels.push(label);
            }
            _ => skipped_labels.push(label),
        }
    }

    if applied_labels.is_empty() {
        return Err(format!(
            "工作表「{sheet_name}」中未识别到任何标准参数，请使用标准输入文件模板"
        ));
    }

    Ok(InputParsePayload {
        values,
        applied_labels,
        skipped_labels,
        sheet_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 标准输入文件模板应解析出全部 30 项标准参数
    #[test]
    fn parses_standard_template() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../public/templates/inputtemplate_ldzl_3.0.xlsx"
        );
        let bytes = std::fs::read(path).expect("模板文件存在");
        let payload = parse_input_xlsx(&bytes).expect("标准模板可解析");

        assert!(payload.sheet_name.to_lowercase().contains("input"));
        assert_eq!(payload.values.len(), INPUT_LABEL_KEYS.len());
        assert_eq!(payload.applied_labels.len(), 30);
        assert_eq!(
            payload.skipped_labels,
            vec!["总评估次数", "初始随机采样点数"]
        );
        assert_eq!(payload.values.get("dod"), Some(&85.0));
        assert_eq!(payload.values.get("windInvest"), Some(&3600.0));
        assert_eq!(payload.values.get("essEnd"), Some(&500.0));
    }

    /// 非标准文件应返回明确错误
    #[test]
    fn rejects_non_template_file() {
        assert!(parse_input_xlsx(b"not an xlsx").is_err());
    }
}
