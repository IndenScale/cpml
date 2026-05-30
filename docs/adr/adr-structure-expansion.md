# ADR: 结构与碰撞语法糖展开

## 背景

Collision 和 Structure 是 CPML 中两种语法糖，自动从简洁声明中生成探针+投影对。需要明确其展开规则。

## 决策

**Collision 展开规则：**

- `collision.hard` → 生成一个探针（断言 `Empty`，场 = 第一个 hard kind 的 OccupancyField，等级 = Error）+ 一个投影（`Contribution::Occupancy { kind: Hard }`）
- `collision.soft` → 生成一个探针（断言 `Empty`，场 = 第一个 soft kind 的 OccupancyField，等级 = Warning）+ 一个投影（`Contribution::Occupancy { kind: Soft }`）
- 自动查找匹配 kind 的 OccupancyField；若未声明该 kind 的场，则报错

**Structure 展开规则：**

- 每个 `structure` 条目生成一个探针和一个投影
- 探针断言：使用 structure 的 `assert` 字段，若未提供则根据场类型推断默认值（Occupancy → `Empty`，Capacity/Scalar → `Gte(0.0)`，Presence → `Present` 空条件）
- 投影贡献：根据场类型从 structure 的 `kind`/`value`/`operator`/`record` 字段构建
- 诊断等级：使用 structure 声明的 `diagnostic_level`，未提供则按场类型取默认值

**ID 生成：** 自动生成的探针/投影 ID 格式为 `"{activity_id}/collision_{hard|soft}_probe"` 和 `"{activity_id}/struct_{name}_projection"`。

## 后果

- 用户可以用 3 行 YAML（一个 collision 条目）替代 20+ 行显式探针+投影声明
- 碰撞语法糖强制要求场段中声明对应 kind 的 OccupancyField
- Structure 的自动断言推断可能在 PresenceField 上产生无意义的默认断言（空 key），需要用户显式指定
