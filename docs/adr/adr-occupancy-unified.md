# ADR: 统一占用场 — Hard/Soft 耦合

## 背景

原始设计将 hard 和 soft 建模为两个独立的 OccupancyField 实例，各自携带 `kind: hard` 或 `kind: soft`。这意味着：

- hard 探针只检查 hard 场 → 只能检测到硬-硬碰撞
- soft 探针只检查 soft 场 → 只能检测到软-软碰撞
- 软-硬碰撞（如塔吊风险区侵入相邻建筑实体）不被任何探针捕获

这与物理直觉不符：实体的风险区侵入另一个实体，应当触发告警；实体的物理边界被风险区覆盖，同样是严重问题。

## 决策

**统一占用场：** 只存在一个 OccupancyField，投影携带 `kind: hard|soft` 标签注入该场。探针不区分 hard/soft——它检查该区域"是否有任何占用"。

**自动诊断定级：** 探测到的重叠按"最严重的 kind"定级：

| 重叠投影的 Kind 组合 | 诊断等级 |
|---|---|
| 任意投影为 Hard | **Error** |
| 全部为 Soft | **Warning**（可覆盖） |

**探针的 `diagnostic_level` 作为下限：** 实际等级 = `max(auto_level, probe.diagnostic_level)`。即：hard 重叠不能被降级，soft 重叠可被用户显式升级为 Error。

### 求值逻辑

```rust
fn eval_occupancy(projections, probe_region, self_id) -> Option<OccupancyKind> {
    // 遍历所有活跃投影
    // Hard → 立即返回 Hard（最严重，无需继续）
    // Soft → 记录，继续查找是否有 Hard
    // 排除自身活动的投影（自排除规则不变）
}
```

### 碰撞语法糖展开

`collision.hard` 和 `collision.soft` 均引用同一个 OccupancyField，只是几何不同：

- `collision.hard`：探针检查 `body` 几何区域 → 默认 diagnostic_level = Error
- `collision.soft`：探针检查 `swing` 几何区域 → 默认 diagnostic_level = Warning

当 soft 探针检测到 hard 投影重叠时，自动升级为 Error。

## 后果

- 软-硬碰撞（风险区侵入实体、实体侵入风险区）被正确检测并报告为 Error
- 不再需要在 fields 段声明两个独立的 occupancy 场——一个 `type: occupancy` 足够
- 如果用户只想检测硬碰撞，只声明 `collision.hard` 即可——此时 soft 区的侵入也会被 hard 探针检测到并报 Error
- 归因（Blame）包含所有重叠的占用投影（hard 和 soft），便于用户判断是物理碰撞还是风险区侵入
