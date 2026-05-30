# ADR: 探针断言模型

## 背景

每个探针声明一个断言，在场状态上求值。断言格式取决于被采样的场类型。

## 决策

**按类型限制的断言：** 编译器在模型构建时验证断言格式与场类型匹配：

| 场类型 | 断言 | 语义 |
|---|---|---|
| Capacity | `Gte(f64)` | 采样值 ≥ 阈值 |
| Scalar | `Gte(f64)` | 采样值 ≥ 阈值 |
| Occupancy | `Empty` | 采样区域必须为空 |
| Presence | `Present(criteria)` | 匹配记录必须存在且有效 |

**默认诊断等级：**

| 场类型 | 默认等级 |
|---|---|
| Capacity | `Error` |
| Scalar | `Error` |
| Occupancy (hard) | `Error` |
| Occupancy (soft) | `Warning` |
| Presence | `Error` |

所有等级均可通过 `diagnostic_level` 在探针级别覆盖。

**校验：** 在占用场上使用 `Gte` 或在容量场上使用 `Empty` 会在模型构建时产生 `TypeMismatchError`。

## 后果

- 用户无法写出无意义的断言（如在容量场上检查"空"）
- `Empty` 断言每次仅检查一种 `kind` 的占用（hard 或 soft），与场的声明 kind 匹配
- 跨 kind 采样（hard 场探针检查 soft 场状态）需要声明独立探针
