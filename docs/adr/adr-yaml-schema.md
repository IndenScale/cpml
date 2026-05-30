# ADR: YAML Schema 设计

## 背景

CPML 使用 YAML 作为序列化格式，`.cpml` 作为文件后缀。Schema 需要兼顾施工工程师的可读性和编译器的可处理性。

## 决策

**四段式扁平布局：** `version`、`name`、`description`、`fields`、`geometries`、`activities`。

- `version`（必填）：版本字符串，用于前向兼容（当前 "0.1"）
- `name`、`description`：可选元数据
- `fields`：字段声明，通过 `type` 区分四种类型（`capacity`、`occupancy`、`scalar`、`presence`）
- `geometries`：命名几何体，通过字符串 ID 被活动引用
- `activities`：核心建模段

**字符串 ID 引用：** 几何体和字段通过字符串 ID 从探针、投影、碰撞条目中引用，在模型构建阶段解析。

**扁平化断言：** 断言在 Rust schema 中使用 `#[serde(flatten)]`，断言字段（`gte`、`empty`、`present`）直接在探针层级内联，而非嵌套在 `assert:` 键下。

```yaml
probes:
  - name: "check_power"
    field: "power_supply"
    geometry: "zone_A"
    gte: 150.0           # 扁平化断言
```

**命名规范：** YAML 键使用 `snake_case`。ID 使用描述性字符串（如 `crane_A_body`、`excavation_zone`）。

**按场类型区分的断言格式：**

| 场类型 | 断言格式 | YAML 键 |
|---|---|---|
| capacity | `{gte: f64}` | `gte` |
| scalar | `{gte: f64}` | `gte` |
| occupancy | `{empty: bool}` | `empty` |
| presence | `{present: {key, type, attributes}}` | `present` |

## 后果

- 扁平化断言使 YAML 更简洁，但可能让习惯嵌套 `assert:` 键的用户困惑
- 基于字符串的引用需要在编译前进行一次解析遍历
- 新增断言类型需要在 `AssertionDef` 枚举中添加变体
