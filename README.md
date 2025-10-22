# 氪-Kr

[<img alt="crates.io" src="https://img.shields.io/crates/v/kr.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/kr)
[<img alt="MIT" src="http://img.shields.io/badge/license-MIT-brightgreen.svg?style=for-the-badge" height="20">](http://opensource.org/licenses/MIT)

[氪-Kr] Rust开发工具包

## 安装

```shell
cargo add kr
```

## kr-core

#### 功能

- AES
  - CBC
  - ECB
  - GCM
- Hash
- 时间格式化
- 基于Redis的分布式锁
- 基于 `bb8` 的Redis异步Manager
- API Code 宏定义：`define_ok!` 和 `define_error_codes!`

⚠️ `aes` 相关功能依赖 `openssl`

## kr-macros

#### 派生宏：Model

- 使用

```rust
#[derive(Model)]
#[partial(UserLite !(email, phone))] // 排除字段
#[partial(UserBrief (id, name), derive(Copy, Debug))] // 包含字段
pub struct User {
    pub id: i64,

    #[sqlx(rename = "username")]
    pub name: String,

    pub email: String,
    pub phone: String,
    pub created_at: String,
    pub updated_at: String,
}
```

- 生成代码

```rust
#[derive(sqlx::FromRow)]
pub struct UserLite {
    pub id: i64,

    #[sqlx(rename = "username")]
    pub name: String,

    pub created_at: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow, Copy, Debug)]
pub struct UserBrief {
    pub id: i64,

    #[sqlx(rename = "username")]
    pub name: String,
}
```

👉 具体使用可以参考 [rnx](https://crates.io/crates/rnx)

**Enjoy 😊**
