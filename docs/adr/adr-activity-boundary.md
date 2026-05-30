# ADR: 活动建模边界 — 拆分准则与活动模板

## 背景

CPML 将施工过程建模为原子活动（Atomic Activity）的集合。每个活动携带 timespan、probe、projection、collision/structure 语法糖。当活动内部状态在时间上变化显著时，通过 activity series 拆分为多个原子活动。

当前设计中，系统对"何时拆分、何时保持原子"没有明确的判断准则，对"是否应定义具有固有风险包络的活动模板"也没有立场。本文给出这两个问题的决策。

## 决策

### 第一部分：原子活动拆分准则

**一个活动应保持为单个原子活动，当且仅当以下四个条件在活动持续期间全部成立：**

| 条件 | 含义 | 违反时的表现 |
|------|------|-------------|
| **空间占用恒定** | 活动的 hard/soft occupancy 几何体不发生位移、缩放或形态变化 | 塔吊变幅、开挖面推进 |
| **资源影响恒定** | 活动对 CapacityField 的注入/消耗速率不变 | 浇筑速率变化、材料消耗加速 |
| **风险水平恒定** | 活动对 ScalarField（如 fire_load）和 soft occupancy（如风险区半径）的贡献不变 | 储油罐安装后火险骤升、试运行期间风险最高 |
| **管理粒度允许** | 活动的时间跨度不超过管理上需要区分的最小时间单位，或活动的内部状态变化对周边活动无差异化影响 | 持续数周的养护，每周强度不同，对后续活动通过/失败判定有不同结果 |

**违反任一条件时，应拆分为多个原子活动（或使用 activity series）：**

```text
# 养护 — 违反"资源影响恒定"（强度每周递增）
- id: "curing_week1"     # 0 → 0.45
- id: "curing_week2"     # 0.45 → 0.8
- id: "curing_week3"     # 0.8 → 1.0
  series: "foundation_curing"

# 道路开挖→埋管→回填 — 违反"空间占用恒定"（沟槽位置推进）
- id: "trench_excavation"  # 占用：开挖段几何
- id: "pipe_laying"        # 占用：埋管段几何
- id: "backfill"           # 占用：回填段几何
  series: "road_works"

# 柴发调试 — 违反"风险水平恒定"（带载测试时火险最高）
- id: "cold_commissioning"   # fire_load = 0.2
- id: "load_test"            # fire_load = 1.0
  series: "dg_commissioning"
```

**活动应保持为单个原子活动，当满足全部四个条件时：**

```text
# 道路封闭 — 空间、资源影响、风险在整个封闭期间恒定，且时间足够短（1周）
- id: "north_road_closure"
  collision:
    hard: { geometry: "north_road" }
  projections:
    - field: "road_access"
      geometry: "north_road"
      value: -100
  # 无需拆分 — 封闭期间的所有内部工序（破路、挖沟、埋管、回填）
  # 对柴发小楼的施工计划而言是"外部扰动"，内部细节不产生差异化影响
```

**活动的时间跨度与管理粒度的关系：**

当活动持续时间远小于方案总工期，且其内部状态变化不产生可被其他活动探针检出的差异时，即使内部有过程，也应保持为原子活动。例如：半天精度的混凝土浇筑——坍落度变化、初凝时间等物理过程确实存在，但施工计划不关心这些，只关心"浇筑完成"这个结果。

### 第二部分：活动模板（Activity Template）

**定义：** 活动模板是一个预声明的、携带完整 probe + projection + structure 包络的活动模式，代表某类施工操作固有的、不可剥离的风险与需求组合。

**判断某类施工操作是否应定义为活动模板的准则：**

活动模板存在的前提是：该操作存在**不可剥离的伴随风险/需求**——即，只要执行这类操作，以下要素必然同时出现，无法通过"换一种方式做"来消除：

1. **固有风险投影** — 操作本身必然产生的风险（如焊接的火源、登高的坠落风险）
2. **固有环境敏感探针** — 操作对特定外部条件的脆弱性（如喷涂对可燃气体浓度的敏感）
3. **固有资源需求探针** — 操作必然需要的资源（如焊工、登高设备）
4. **固有空间需求投影+探针** — 作业净空、安全距离

**示例：钢结构焊接活动模板**

```yaml
# 模板定义（概念层，尚未确定 DSL 语法）
activity_template:
  name: "steel_welding"
  description: "钢结构焊接 — 固有火源 + 可燃气体敏感 + 登高风险 + 焊工需求"

  # 不可剥离的固有投影
  inherent_projections:
    - name: "arc_fire_source"
      field: "fire_load"
      geometry: "weld_point"       # 调用时替换为实际位置
      value: 0.6                   # 电弧火源
      geometry_shape: "sphere"
      geometry_params: { radius: 0.5 }  # 小球状火源

    - name: "slag_fire_source"
      field: "fire_load"
      geometry: "weld_area_below"
      value: 0.4                   # 熔渣坠落
      geometry_shape: "cylinder"
      geometry_params: { radius: 1.5, half_height: 0.3 }

    - name: "height_risk"
      field: "occupancy"
      geometry: "fall_zone"
      kind: soft                   # 坠落风险区 — soft occupancy
      geometry_shape: "sphere"
      geometry_params: { radius: 3.0 }

  # 不可剥离的固有探针
  inherent_probes:
    - name: "combustible_gas_check"
      field: "combustible_gas"
      geometry: "weld_zone"
      lte: 0.3
      diagnostic_level: warning    # 封闭空间喷漆/涂装产生的 VOC

    - name: "clearance_check"
      field: "occupancy"
      geometry: "work_envelope"
      empty: true
      diagnostic_level: error      # 作业净空 — 必须满足

    - name: "height_work_info"
      field: "occupancy"           # 或专用 risk 场
      geometry: "fall_zone"
      # 无失败条件 — 纯 info 级别，作为交底提醒
      # 登高风险不可消除，仅做记录
      diagnostic_level: info       # 应用层可过滤

  # 不可剥离的固有资源需求
  inherent_demands:
    - resource: "certified_welder"
      count: 2
    - resource: "height_access_equipment"  # 登高设备或悬挂点
      count: 1
```

**为什么模板不是语法糖：**

`collision` 和 `structure` 是语法糖——它们是对已有原语（probe/projection）的简写，展开后完全等价于显式声明。

活动模板**不是**语法糖。它引入的是知识的缺省填充——施工计划工程师可能遗漏"焊接→火源→可燃气体敏感"的因果链，但物理规律不会遗漏。模板把领域知识固化为可复用的包络，使遗漏在 CPML 编译期暴露而非在现场暴露。

**模板与原子活动的关系：**

模板实例化后产生一个（或一组）原子活动。实例化时需要填入：

- 具体的时间和位置（`timespan`、`geometry` 引用）
- 可覆盖的参数（如焊工数量，模板提供默认值）
- 可追加的额外探针/投影（特定场景的特殊需求）

模板是否违反原子活动拆分准则？不违反——如果焊接过程的四个条件恒定（同一位置、同一火源强度、同一风险水平），它就是一个原子活动。模板只是确保这个原子活动**不会因为计划工程师的知识盲区而变成不完整的描述**。

### 第三部分：受众视角与多文件建模

**问题：** 前两部分的拆分准则和模板机制隐含假设了一个单一全知视角——EPC 总包对全场所有活动都有同等分辨率的需求。但真实施工组织中，不同受众对同一物理过程的关注粒度不同：

| 受众 | 对柴发小楼的关注 | 对主楼/管廊的关注 |
|------|-----------------|-------------------|
| EPC 总包 | 全细节：基础强度、钢结构、柴发吊装、消防、配电 | 全细节 |
| 柴发分包商 | 全细节 | **粗略阻挡盒**——只知道"那个方向有个大体量，这段时间占着" |
| 道路分包商 | 不关心 | 全细节（管廊开挖），柴发楼只是**运料终点** |
| 管廊分包商 | 不关心内部，只关心**接口点位置和容量** | 全细节 |

这意味着"是否拆分"的判断不能仅依赖四条件，还需要考虑第五个维度：

**5. 受众相关性** — 活动的内部状态变化是否影响该受众的决策或风险？

```text
# 对柴发分包商而言：
#   主楼基础开挖的强度检测 —— 不相关，不需要知道
#   主楼占用空间 + 时间段 —— 相关，但只需要 occupancy 阻挡盒

# 对 EPC 总包而言：
#   主楼基础开挖的强度检测 —— 相关，上层钢结构安装的前置条件
```

**同一物理活动，在不同受众的 .cpml 中是不同的建模精度：**

```text
# EPC 的 main_building.cpml（全细节）
- id: "main_building_foundation"
  structures:
    - field: "concrete_strength"
      geometry: "mb_foundation"
      value: 0.0
  # ... 完整的养护 series、强度探针

# 柴发分包商的 dg_building.cpml（主楼作为外部扰动）
- id: "ext_main_building"
  name: "主楼（外部）"
  timespan: { start: "2026-03-01", end: "2026-09-30" }
  collision:
    hard: { geometry: "main_building_envelope" }
  # 没有 structures、probes、内部细节
  # 就是一个"挡在那里的盒子"
```

**多文件编译模型：**

CPML 应支持每个受众维护独立的 `.cpml` 文件，编译时通过交叉引用（cross-reference）机制链接：

1. **公共几何体注册表** — 所有受众共享的几何体定义（场地边界、道路、主要结构外轮廓），避免重复定义和坐标系不一致
2. **外部活动声明** — 文件中可以声明 `external` 活动，表示"这不是我管的，但我知道它在这段时间占据这个空间/消耗这个资源/产生这个风险"
3. **编译视角选择** — 编译器以某个受众文件为主文件（primary），将其 external 活动视为该受众视野中的外部扰动，不检查 external 活动的内部一致性（那是别的受众的事），但检查 external 活动对本受众活动的交叉影响

```yaml
# dg_building.cpml（柴发分包商视角，主文件）
version: "1.0"
name: "柴发小楼施工计划 — 柴发分包商视角"

# 引用公共几何体
includes:
  - "common/geometries.cpml"

# 外部活动（其他分包商/EPC 的活动，作为阻挡盒）
external_activities:
  - id: "ext_main_building"
    source: "main_building.cpml"     # 来源文件
    timespan: ...                     # 可覆盖时间（根据分包商获得的最新信息）
    collision:
      hard: { geometry: "main_building_envelope" }

  - id: "ext_utility_tunnel"
    source: "utility_tunnel.cpml"
    collision:
      hard: { geometry: "utility_tunnel" }

# 本分包商自己的活动（全细节）
activities:
  - id: "dg_1f_placement"
    # ... 完整的 probe + projection + structure
```

**受众粒度的级联效应：**

当 EPC 更新主楼施工计划后，柴发分包商可以选择：

- **仅更新 external 活动的 timespan**（主楼延期了，阻挡时间变了）
- **保持 external 活动内部结构不感知**（柴发分包商不需要知道延期是因为基础养护慢了还是钢筋到货晚了）

这正是"统一接口原则"（`adr-fields.md`）在受众维度的延伸——外部信息通过活动进入 CPML，而多文件模型使得"外部"的定义取决于受众视角。

### 非目标

以下**不是**活动模板的用途：

- **可配置的施工模式**（如"标准层施工"）：不同项目的标准层施工差异太大，模板化会限制灵活性。应通过常规 activity 定义 + 人工或 AI 辅助生成。
- **WBS 层级模板**：CPML 暂不引入活动层级，不定义父子关系。
- **工序依赖链模板**（如"基础→柱→梁→板"的固定序列）：依赖关系因项目而异，不适合模板化。

## 后果

1. **原子活动拆分准则**为计划工程师和 LLM 生成 CPML 提供了明确的判断边界：四个物理条件 + 一个受众条件 → 决定拆分还是保持原子。
2. **活动模板**将领域知识编码为机器可检查的约束。焊接活动的完整风险包络被编译器强制携带，不能通过"忘记声明 fire_load 投影"来跳过。
3. **多文件建模**使每个受众只需维护自己关心的精度。EPC 维护全细节主文件，分包商维护自己的细节文件 + 他人的粗略阻挡盒。外部活动更新（延期、范围变化）通过覆盖 timespan/geometry 实现，不穿透到不相关的内部细节。
4. 多文件的实现成本：编译器需支持 `includes`（几何体共享）和 `external_activities`（跨文件活动引用）两个新机制。external 活动的语义与常规活动不同——仅参与 occupancy/capacity 场注入，不要求内部 probe 通过。
5. 模板实例化参数（时间、位置、覆盖值）是纯 YAML 替换，不引入新的编译器概念——模板展开发生在上游（CPML 生成时），而非编译器中。
