# ADR: 诊断结构

## 背景

当探针断言失败时，编译器需要产生可操作的诊断信息，帮助用户理解发生了什么问题及其根因。

## 决策

**诊断结构：**

```rust
struct Diagnostic {
    keyframe_index: usize,     // 哪个关键帧
    keyframe_date: NaiveDate,  // 日历日期
    activity_id: String,       // 哪个活动的探针失败
    probe_id: String,          // 哪个具体探针
    level: DiagnosticLevel,    // debug/info/warning/error/fatal
    message: String,           // 人类可读的描述
    blame: Vec<BlameEntry>,    // 贡献投影列表
}

struct BlameEntry {
    activity_id: String,       // 哪个活动贡献了
    projection_id: String,     // 哪个具体投影
    contribution_summary: String, // 如 "capacity(-150)"、"occupancy(Hard)"
}
```

**归因追溯：** 当探针失败时，编译器收集同场上所有投影（跨所有活动，非仅活跃活动），并将其作为归因条目。这让用户看到场贡献者的完整图景。

**排序：** 诊断先按严重度降序排列，同严重度内按关键帧索引排序。

**消息模板：** 消息根据断言类型和采样值生成：

- Occupancy: `"Occupancy collision detected: region is occupied ({kind:?} kind)"`
- Value: `"Value below threshold: sampled {value} < required {threshold} on field '{field}'"`
- Presence: `"Required presence record '{key}' not found or invalid on field '{field}'"`

**退出码：** 如有任何诊断等级 ≥ Error，CLI 退出码为 1。

## 后果

- 归因条目包含场上所有投影（capacity/scalar/presence），包括留下持久状态的非活跃活动
- 按比例的归因（如"此投影贡献了场值的 40%"）推迟到 v2
- 消息格式固定；国际化需要单独的消息层
