#[path = "socket/redis.rs"]
mod redis;
#[path = "socket/tcp.rs"]
mod tcp;

pub(crate) use self::redis::invoke_redis_recipe;
pub(crate) use self::tcp::invoke_tcp_recipe;
