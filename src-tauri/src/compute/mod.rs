//! 计算核心模块（全部计算流程在 Rust 后端执行，确保准确性）
//!
//! - `params`：参数模型与后端权威校验（FR-5）
//! - `simulate`：8760h 逐时仿真引擎（AR-2 九项约束）
//! - `economics`：经济性计算与目标函数（AR-1 式 3/4、方案一/二）
//! - `ga`：遗传算法求解器（AR-3）
//! - `engine`：总引擎，产出前端展示与报告导出所需的全部结果数据

pub mod economics;
pub mod engine;
pub mod ga;
pub mod params;
pub mod simulate;

pub use engine::ComputeResultPayload;
pub use params::{ComputeParams, CurveData};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod golden;
