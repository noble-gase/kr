# 氪-Kr

[<img alt="crates.io" src="https://img.shields.io/crates/v/kr.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/kr)
[<img alt="MIT" src="http://img.shields.io/badge/license-MIT-brightgreen.svg?style=for-the-badge" height="20">](http://opensource.org/licenses/MIT)

[氪-Kr] Rust开发实用库

## 安装

```shell
cargo add kr
```

## 模块

### kr-core

> 核心模块

- AES
  - CBC
  - ECB
  - GCM
- Hash
- 时间格式化
- 基于Redis的分布式锁
- 基于 `bb8` 的Redis异步Manager
- 生成API错误码的宏：`define_ok!` 和 `define_error_codes!`

⚠️ `aes` 相关功能依赖 `openssl`

### kr-macros

> 宏定义模块

#### Model 宏

- 使用

```rust
#[derive(Model)]
#[partial(UserLite !(email, phone))] // 排除字段
#[partial(UserBrief (id, name), derive(Copy, Debug))] // 包含字段
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub created_at: String,
}
```

- 生成代码

```rust
#[derive(sqlx::FromRow)]
pub struct UserLite {
    pub id: i64,
    pub name: String,
    pub created_at: String,
}

#[derive(Copy, Debug, sqlx::FromRow)]
pub struct UserBrief {
    pub id: i64,
    pub name: String,
}
```

👉 具体使用可以参考 [rnx](https://crates.io/crates/rnx)

**Enjoy 😊**
