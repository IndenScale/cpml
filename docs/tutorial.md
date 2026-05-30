# CPML 开发者教程

本文档面向 CPML 的开发者与使用者，渐进式展开 CPML 的全部概念。每节包含可运行的示例，建议按顺序阅读。

> **前置要求**: 已安装 Rust 工具链。`cargo build --release` 后得到 `./target/release/cpml` 二进制文件。

---

## 1. 什么是施工计划 / 施工仿真

### 1.1 施工计划

一个施工计划回答三个问题：

- **谁**在**什么时间**、在**哪里**、做**什么**？
- 这些活动之间有什么**依赖关系**（谁必须在谁之前完成）？
- 它们共享哪些**资源**（电力、道路、塔吊、班组），会不会**冲突**？

传统施工计划是一张 Gantt 图——时间线和活动名字。它告诉你"基础浇筑是 3 月 12 日到 3 月 14 日"，但不会告诉你"3 月 13 日那天，塔吊 A 的回转半径会扫过正在养护的基础，而基础强度只有设计值的 45%，如果塔吊吊着重物经过上方……"。

### 1.2 施工仿真

施工仿真把 Gantt 图推进到**空间+物理+资源**层面：

```text
Gantt 图:      "3月12日 浇筑基础"
施工仿真:      "3月12日 08:00-17:00, 混凝土泵车占据坐标(15, 7.5)半径5m,
                消耗电力150kW, 向基础场注入0.2强度,
                与塔吊A的软碰撞区(半径25m)重叠 → WARNING"
```

CPML 做的是**静态分析**——给定一个已排定的施工计划，逐时间点检查所有活动的空间占用、资源消耗、状态依赖是否自洽。它不帮你排计划，但它告诉你排出来的计划哪里会出问题。

### 1.3 为什么这很重要

施工行业的变更成本随阶段指数增长。设计阶段改一根梁的成本是 1，施工阶段改就是 100。在纸面上发现"你的塔吊会撞到我的脚手架"的成本，远低于在现场发现。

---

## 2. 第一个 CPML 施工计划

### 2.1 最小模型：几何 + 活动

CPML 文件是一个 YAML 文档，至少需要三样东西：

```yaml
version: "1.0"
name: "最小施工计划"

geometries: # 空间中有什么
  - id: "site"
    cuboid: { half_extents: [20, 20, 5] }
    pose: { position: [10, 10, 5] }

activities: # 谁在什么时间做什么
  - id: "excavation"
    name: "基础开挖"
    timespan:
      start: "2026-03-01"
      end: "2026-03-05"
```

保存为 `plan.cpml`，运行：

```bash
./target/release/cpml check plan.cpml
```

输出：

```text
=== CPML Compilation Report ===
Project: 最小施工计划
Total diagnostics: 0
Schedule duration: 4 days
```

没有诊断——因为还没有声明任何可能冲突的东西。

### 2.2 几何体

CPML 支持 6 种几何体素 + 3 种布尔组合：

```yaml
geometries:
  - id: "tower_body"
    cuboid: { half_extents: [2.5, 2.5, 15] } # 长方体
  - id: "tower_swing"
    sphere: { radius: 25.0 } # 球体
  - id: "fuel_tank"
    cylinder: { radius: 1.25, half_height: 3.0 } # 圆柱
  - id: "silo_top"
    hemisphere: { radius: 2.0 } # 半球
  - id: "hopper"
    cone: { radius: 1.5, half_height: 2.0 } # 锥体

  # 布尔组合
  - id: "L_shaped_building"
    union: # 并集
      - cuboid: { half_extents: [20, 10, 5] }
      - cuboid: { half_extents: [10, 20, 5] }
    pose: { position: [30, 20, 5] }

  - id: "wall_with_opening"
    subtract: # 差集 (A - B)
      a:
        cuboid: { half_extents: [10, 0.5, 5] }
      b:
        cuboid: { half_extents: [2, 1, 2.5] }
```

`pose` 定义位置与旋转（Euler 角，ZYX 顺序，度数）：

```yaml
pose:
  position: [15, 7.5, 5] # [x, y, z] 世界坐标
  rotation: [0, 0, 0] # [yaw, pitch, roll] 默认零
```

### 2.3 活动

活动是施工过程的基本建模单元。每个活动至少需要 `id` 和 `timespan`：

```yaml
activities:
  - id: "rebar_install"
    name: "钢筋绑扎"
    timespan:
      start: "2026-03-10T08:00" # 支持半天精度
      end: "2026-03-12T17:00"

  - id: "concrete_pour"
    name: "混凝土浇筑"
    timespan:
      start: "2026-03-12T08:00"
      end: "2026-03-12T17:00" # 子日精度
    depends_on: # 工序依赖
      - activity_id: "rebar_install"
        kind: FS # Finish-to-Start
```

支持的依赖类型：`FS`（Finish-to-Start，默认）、`SS`、`FF`、`SF`，可选 `lag_days`。

---

## 3. 运行编译器

### 3.1 基本命令

```bash
# 仅解析（验证 YAML 结构）
cpml parse plan.cpml

# 完整检查（解析 + 展开 + 关键帧求值 + 诊断）
cpml check plan.cpml

# 多方案对比
cpml check plan_a.cpml --output-json > a.json
cpml check plan_b.cpml --output-json > b.json
cpml compare plan_a.cpml plan_b.cpml
```

### 3.2 理解输出

编译器输出包含：

- **Diagnostics** — 按严重度排列（Fatal > Error > Warning > Info > Debug），每条包含 keyframe 时间、活动 ID、探针 ID、失败原因、Blame 归因
- **Metrics** — risk_index（风险指数）和 cost_impact（成本影响）的时间序列
- **Schedule duration** — 总工期（天）

---

## 4. 场、探针、投影

### 4.1 为什么需要建模相互影响

回到第 2 节的例子——两个活动的时间重叠了，但 CPML 没有报告任何冲突。为什么？因为我们只告诉了 CPML "什么时间有什么活动"，没有告诉它：

- 每个活动**占据多少空间**（投影）
- 每个活动**需要什么条件**（探针）
- 活动之间通过什么**介质**相互影响（场）

这三个概念是 CPML 的核心抽象：

```text
活动 A                   活动 B
  │                        │
  ├─ projection ──→ 场 ←──┤  (A 向场注入影响)
  │                        │
  │                  probe ←┤  (B 从场采样，检查条件)
```

场是活动之间**唯一的通信渠道**。两个活动不直接通信——它们通过场来感知彼此的存在和影响。

### 4.2 OccupancyField — 空间碰撞

**概念：** 建模几何占用——物理碰撞（硬碰撞）和风险/安全区域（软碰撞）。

**场的值域：** 空间点上的布尔值——`占用` 或 `空`。任一投影声明该点即视为占用。

**完整示例：** `samples/collision_demo.cpml`

两座塔吊，各自携带 hard collision（塔身）和 soft collision（摆臂半径）。编译器检测到硬碰撞（塔身重叠）→ Error，软碰撞（摆臂区重叠）→ Warning。

```yaml
fields:
  - name: "crane_occupancy"
    type: occupancy

geometries:
  - id: "crane_A_body"
    cuboid: { half_extents: [2.5, 2.5, 15] }
    pose: { position: [2.5, 2.5, 15] }
  - id: "crane_A_swing"
    sphere: { radius: 25.0 }
    pose: { position: [2.5, 2.5, 25] }
  - id: "crane_B_body"
    cuboid: { half_extents: [2.5, 2.5, 15] }
    pose: { position: [14.5, 2.5, 15] }

activities:
  - id: "crane_A"
    timespan: { start: "2026-03-12", end: "2026-03-20" }
    probes:
      - field: "crane_occupancy"
        geometry: "crane_A_body"
        empty: true # 断言：这里必须是空的
        diagnostic_level: error
    projections:
      - field: "crane_occupancy"
        geometry: "crane_A_body"
        kind: hard

  - id: "crane_B"
    timespan: { start: "2026-03-12", end: "2026-03-20" }
    probes:
      - field: "crane_occupancy"
        geometry: "crane_B_body"
        empty: true
        diagnostic_level: error
    projections:
      - field: "crane_occupancy"
        geometry: "crane_B_body"
        kind: hard
```

**三阶段碰撞检测：**

1. AABB 快速剔除 → 2. GJK 精确相交测试 → 3. 遮挡剔除（仅对 soft 碰撞，ray-casting 检查屏障）

**遮挡剔除：** `samples/occlusion_demo.cpml` — 两座塔吊之间有墙体 barrier，软碰撞被阻挡，但硬碰撞不受影响（物理碰撞不能被"遮挡"）。

### 4.3 CapacityField — 资源容量

**概念：** 建模有限的、可消耗的资源——电力、供水、通道通行量、材料库存、劳动力。

**场的值域：** 实数。正值 = 供给，负值 = 消耗。叠加算子：代数和。

**完整示例：** `samples/resource_contention.cpml`

```yaml
fields:
  - name: "power_supply"
    type: capacity

activities:
  # 供电活动 — 注入 500kW
  - id: "power_provision"
    timespan: { start: "2026-03-01", end: "2026-03-30" }
    projections:
      - field: "power_supply"
        geometry: "site"
        value: 500

  # 施工活动 A — 消耗 150kW
  - id: "work_A"
    timespan: { start: "2026-03-05", end: "2026-03-07" }
    probes:
      - field: "power_supply"
        geometry: "zone_A"
        gte: 150 # 断言：必须有 ≥ 150kW
        diagnostic_level: error
    projections:
      - field: "power_supply"
        geometry: "zone_A"
        value: -150 # 消耗 150kW

  # 施工活动 B — 同时消耗 200kW
  - id: "work_B"
    timespan: { start: "2026-03-05", end: "2026-03-07" }
    probes:
      - field: "power_supply"
        geometry: "zone_B"
        gte: 200
    projections:
      - field: "power_supply"
        geometry: "zone_B"
        value: -200
```

3 月 5 日 - 7 日：500 - 150 - 200 = 150。work_B 需要 200 → Error，Blame 指向 power_provision（供给不足）和 work_A（消耗了一部分）。

### 4.4 ScalarField — 状态递进

**概念：** 建模空间中随时间累积/变化的标量值——混凝土强度、钢结构完成度、沉降量。

**场的值域：** 非负实数。支持四种算子：`max`（取最大值）、`min`、`sum`（累加）、`replace`（覆盖）。**关键特性：持久化**——投影活动结束后值继续存在。

**完整示例：** `samples/scalar_progression.cpml`

```yaml
fields:
  - name: "concrete_strength"
    type: scalar
    operator: max # 取历史最大值

activities:
  # 钢筋 → 强度 0
  - id: "rebar"
    timespan: { start: "2026-03-10", end: "2026-03-12" }
    projections:
      - field: "concrete_strength"
        geometry: "foundation"
        value: 0.0
        operator: replace

  # 浇筑 → 强度 0.2
  - id: "pour"
    timespan: { start: "2026-03-12", end: "2026-03-13" }
    depends_on:
      - activity_id: "rebar"
        kind: FS
    projections:
      - field: "concrete_strength"
        geometry: "foundation"
        value: 0.2
        operator: max

  # 养护 week 1 → 强度 0.45
  - id: "curing_w1"
    timespan: { start: "2026-03-13", end: "2026-03-20" }
    depends_on:
      - activity_id: "pour"
        kind: FS
    projections:
      - field: "concrete_strength"
        geometry: "foundation"
        value: 0.45
        operator: max

  # 上层施工 → 检查强度 ≥ 0.7
  - id: "column_construction"
    timespan: { start: "2026-03-28", end: "2026-04-05" }
    depends_on:
      - activity_id: "curing_w1"
        kind: FS
    probes:
      - field: "concrete_strength"
        geometry: "foundation"
        gte: 0.7
        diagnostic_level: error
```

3 月 28 日 column_construction 开始时，concrete_strength = max(0, 0.2, 0.45) = 0.45 < 0.7 → Error。需要再等一周养护。

### 4.5 PresenceField — 在场记录

**概念：** 建模带属性的标记实体的存在——许可、审批、中间产物完成、物料到场、设备就位。

**场的值域：** 存在记录的集合，每条记录有 key、type、attributes、有效期。

**完整示例：** `samples/presence_permit.cpml`

```yaml
fields:
  - name: "permits"
    type: presence

activities:
  # 审批活动 — 注入许可
  - id: "permit_approval"
    timespan: { start: "2026-03-01", end: "2026-03-05" }
    projections:
      - field: "permits"
        geometry: "site_wide"
        record:
          key: "excavation_permit"
          type: "permit"
          attributes: { zone: "A", status: "approved" }
          valid_until: "2026-06-30"

  # 开挖活动 — 检查许可存在
  - id: "excavation"
    timespan: { start: "2026-03-06", end: "2026-03-10" }
    depends_on:
      - activity_id: "permit_approval"
        kind: FS
    probes:
      - field: "permits"
        geometry: "site_wide"
        present:
          key: "excavation_permit"
          type: "permit"
          attributes: { status: "approved" }
        diagnostic_level: error
```

许可有效期到 6 月 30 日 → 3 月 6 日开挖时通过检查。如果开挖推迟到 7 月 → 许可过期 → Error。

### 4.6 RateField — 速率监控

**概念：** 建模流率——材料进场速率、混凝土供应速率、施工推进速度。通过滑动窗口计算单位时间的变化量，支持背压（Lte）和饥饿（Gte）检测。

**完整示例：** `samples/ratefield_demo.cpml`

```yaml
fields:
  - name: "material_flow"
    type: rate
    window_size: 5 # 5 个关键帧的滑动窗口

activities:
  # 供应商 — 注入高流速
  - id: "fast_supplier"
    timespan: { start: "2026-03-13", end: "2026-03-14" }
    projections:
      - field: "material_flow"
        geometry: "delivery_zone"
        value: 200 # 每关键帧间隔 200 单位

  # 下游消费者 — 检查流速是否过高（背压）
  - id: "downstream_team"
    timespan: { start: "2026-03-13", end: "2026-03-14" }
    probes:
      - field: "material_flow"
        geometry: "delivery_zone"
        lte: 150 # 断言：流速不能超过 150
        diagnostic_level: warning
```

速率 200 > 阈值 150 → Warning（背压）。Blame 指向 `fast_supplier`（注入流速过高）。

### 4.7 断言类型速查

| 断言                        | 适用场类型             | 含义                |
| --------------------------- | ---------------------- | ------------------- |
| `empty: true`               | Occupancy              | 该区域必须为空      |
| `gte: N`                    | Capacity, Scalar, Rate | 值必须 ≥ N          |
| `lte: N`                    | Capacity, Scalar, Rate | 值必须 ≤ N          |
| `range: {min, max}`         | Capacity, Scalar, Rate | 值必须在 [min, max] |
| `present: {key, type, ...}` | Presence               | 记录必须存在且有效  |

---

## 5. 语法糖

语法糖的存在理由是：当一种写法出现的频率足够高、模式足够固定时，编译器应该替用户展开细节，而不是让用户每次手写 20 行。

### 5.1 `collision` — 空间占用

**解决的问题：** 每个活动声明空间占用需要写一个 occupancy probe（检测冲突）+ 一个 occupancy projection（声明占用），共 12 行。但这对模式是**对称且不可分割的**——声明占用就必须同时声明检测。

```yaml
# 展开前（2 行）
collision:
  hard:
    geometry: "crane_body"
  soft:
    geometry: "crane_swing"

# 等价于手写（24 行）
#   hard → probe { empty: true, diagnostic_level: error }
#        + projection { kind: hard }
#   soft → probe { empty: true, diagnostic_level: warning }
#        + projection { kind: soft }
```

### 5.2 `structure` — 对称探针-投影

**解决的问题：** 很多活动既要"向场注入某物"又要"检查场中该物的状态"。比如混凝土养护既要注入强度值，也要检查基础强度是否达标。structure 自动生成这对。

```yaml
# 展开前（3 行）
structures:
  - field: "concrete_strength"
    geometry: "foundation"
    value: 0.45

# 等价于手写（8 行）
#   probe  { gte: 0.0, diagnostic_level: error }
#   projection { value: 0.45, operator: max }
```

`structure` 对 OccupancyField 自动推断 `empty: true` 断言（检查是否被占用），对 ScalarField 推断 `Gte(0.0)`（检查场是否存在）。对 PresenceField 必须显式提供 `assert`（参见 ADR: adr-structure-expansion.md）。

### 5.3 `demands` — 资源消耗

**解决的问题：** 资源消耗是最高频的模式之一——每个需要电力/水/人力的活动都要写"我需要至少 X"（probe, `gte: X`）和"我消耗 X"（projection, `value: -X`）。这两个声明**必须保持一致**，但分开写容易遗漏或写错。

```yaml
# 展开前（4 行）
demands:
  - field: "power_supply"
    geometry: "zone_A"
    amount: 150

# 等价于手写（8 行）
#   probe  { gte: 150, diagnostic_level: error }
#   projection { value: -150 }
```

`amount` 只接受正数；投影自动取负（代表消耗）。diagnostic_level 默认为 Error，可覆盖为 Warning。

### 5.4 语法糖的思想

```text
语法糖 ←→ "你已经知道要写什么，我帮你少写点"
活动模板 ←→ "你不知道要写什么，我帮你补全"
```

语法糖的展开在编译器的 pipeline stage 4（expand）阶段完成，展开后与手写完全等价。它不引入新的运行时语义，纯粹是代码生成。

---

## 6. 高级活动建模

### 6.1 活动模板 — 不可剥离风险包络

**问题：** 某些施工操作**固有地携带风险与需求**，不可剥离。钢结构焊接不仅是"占据空间 3 天"，它必然同时产生：

- 电弧火源（球体 fire_load 0.6）
- 熔渣火源（柱体 fire_load 0.4）
- 可燃气体敏感（如果附近有喷漆/涂装）
- 登高坠落风险（soft occupancy）
- 焊工需求（certified_welder ≥ 2）
- 作业净空需求

如果计划工程师只写 `collision.hard`，遗漏的不是"建模不完整"——是物理规律不允许遗漏。活动模板将这些不可剥离要素预定义为可复用的包络。

**模板目录：** `templates/`

| 模板     | 风险域           | 文件                                      |
| -------- | ---------------- | ----------------------------------------- |
| 电弧焊接 | fire, fall       | `templates/fire/welding_arc.cpml`         |
| 登高作业 | fall             | `templates/fall/work_at_height.cpml`      |
| 吊装作业 | mechanical, fall | `templates/mechanical/lift_crane.cpml`    |
| 燃油操作 | fire             | `templates/fire/fuel_handling.cpml`       |
| 带载测试 | energy, fire     | `templates/energy/commissioning_hot.cpml` |

**使用方式：** 模板使用 `{{param}}` 占位符。实例化在上游（LLM 或 CLI）完成，替换为具体的时间、几何引用。编译器不引入新概念。

参见 `tests/template_validation.rs` — 验证焊接模板可正确实例化，且能检测到附近喷漆产生的可燃气体风险。

### 6.2 系列活动 (Activity Series) — 时变行为

**问题：** 单个原子活动假设其空间占用、资源影响、风险水平在持续期间**恒定不变**。当这些条件不满足时，需要拆分为一组串联的原子活动，通过 `series` 字段关联。

```yaml
# 养护 — 违反"资源影响恒定"（强度每周递增）
- id: "curing_w1"
  series: "foundation_curing"
  timespan: { start: "2026-03-13", end: "2026-03-20" }
  projections:
    - field: "concrete_strength"
      geometry: "foundation"
      value: 0.45
      operator: max

- id: "curing_w2"
  series: "foundation_curing"
  timespan: { start: "2026-03-20", end: "2026-03-27" }
  projections:
    - field: "concrete_strength"
      geometry: "foundation"
      value: 0.8
      operator: max

- id: "curing_w3"
  series: "foundation_curing"
  timespan: { start: "2026-03-27", end: "2026-04-03" }
  projections:
    - field: "concrete_strength"
      geometry: "foundation"
      value: 1.0
      operator: max
```

`series` 有两个约束：

1. **自排除** — 同一 series 内的活动在 OccupancyField 上相互排除（不会自己碰撞自己）
2. **时间不重叠** — 编译器验证同一 series 内的活动 timespan 不能重叠

### 6.3 拆分准则 — 什么是一个原子活动

**一个活动应保持为单个原子活动，当且仅当以下条件全部成立：**

| 条件         | 含义                            | 违反示例                 |
| ------------ | ------------------------------- | ------------------------ |
| 空间占用恒定 | 几何体不位移/变形               | 塔吊变幅、开挖面推进     |
| 资源影响恒定 | 对 CapacityField 的速率不变     | 浇筑速率变化             |
| 风险水平恒定 | fire_load 和 soft zone 半径不变 | 储油罐安装后火险骤升     |
| 管理粒度允许 | 内部变化对周边活动无差异化影响  | 养护每周强度不同         |
| 受众相关性   | 内部细节不影响该受众的决策      | 分包商不关心主楼基础强度 |

**违反任一条件 → 拆分为 activity series。**

**全部成立 → 保持为单个原子活动。**

例如：道路封闭——空间、资源、风险在整个封闭期间恒定，时间短（1 周），封闭期间的内部工序（破路、挖沟、埋管）对柴发小楼的施工计划是外部扰动 → 一条 activity 足够。

详细讨论见 `docs/adr/adr-activity-boundary.md`。

---

## 下一步

- `samples/` — 完整的功能演示文件
- `docs/adr/` — 架构决策记录
- `docs/TECH_DEBT.md` — 技术债追踪
- `docs/workflow.md` — 编译管线流程图
