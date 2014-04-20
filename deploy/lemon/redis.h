#include <hiredis/hiredis.h>

void redisDisconnect(redisContext *ctx)
{
    close(ctx->fd);
    redisFree(ctx);
}

std::string getRedisString(redisReply *reply)
{
    std::string redisString;

    if (reply && reply->type == REDIS_REPLY_STRING)
    {
        redisString = reply->str;
        redisFree(reply);
    }
    else return "";

    return redisString;
}