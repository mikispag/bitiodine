#include <hiredis/hiredis.h>

void redisDisconnect(redisContext *ctx)
{
    close(ctx->fd);
    redisFree(ctx);
}

void displayRedisReply(redisReply *reply)
{

    if (!reply) {
        return;
    }

    switch (reply->type)
    {
    case REDIS_REPLY_STATUS:
        printf("REDIS_REPLY_STATUS: %s\n", reply->str);
        break;
    case REDIS_REPLY_ERROR:
        printf("REDIS_REPLY_ERROR: %s\n", reply->str);
        break;
    case REDIS_REPLY_INTEGER:
        printf("REDIS_REPLY_INTEGER: %lld\n", reply->integer);
        break;
    case REDIS_REPLY_NIL:
        printf("REDIS_REPLY_NIL\n");
        break;
    case REDIS_REPLY_STRING:
        printf("REDIS_REPLY_STRING: %s\n", reply->str);
        break;
    case REDIS_REPLY_ARRAY:
        printf("REDIS_REPLY_ARRAY\n");
        break;
    default:
        break;
    }
}

std::string getRedisString(redisReply *reply)
{
    std::string redisString;

    if (reply && reply->type == REDIS_REPLY_STRING)
    {
        redisString = reply->str;
    }
    else return "";

    return redisString;
}