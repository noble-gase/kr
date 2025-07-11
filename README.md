# 氪-Kr

[<img alt="crates.io" src="https://img.shields.io/crates/v/kr.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/kr)
[<img alt="MIT" src="http://img.shields.io/badge/license-MIT-brightgreen.svg?style=for-the-badge" height="20">](http://opensource.org/licenses/MIT)

[氪-Kr] Rust开发实用库

## 安装

```shell
cargo add kr
```

## 包含

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

👉 具体使用可以参考 [rnx](https://crates.io/crates/rnx)

**Enjoy 😊**
