pub mod async_red_lock;
pub mod red_lock;

pub const SCRIPT: &str = r#"
if redis.call("GET", KEYS[1]) == ARGV[1] then
	return redis.call("DEL", KEYS[1])
else
	return 0
end
"#;
