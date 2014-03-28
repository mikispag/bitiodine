// Dump all addresses ever used in the blockchain

#include <util.h>
#include <common.h>
#include <errlog.h>
#include <option.h>
#include <rmd160.h>
#include <callback.h>

#include <vector>
#include <string.h>

struct Addr
{
    uint160_t hash;
};

static inline Addr *allocAddr() {
    return (Addr*)PagedAllocator<Addr>::alloc();
}

static uint8_t emptyKey[kRIPEMD160ByteSize] = { 0x52 };
typedef GoogMap<Hash160, Addr*, Hash160Hasher, Hash160Equal>::Map AddrMap;

struct AllAddresses:public Callback
{
    int64_t limit;
    int64_t cutoffBlock;
    optparse::OptionParser parser;

    AddrMap addrMap;
    uint32_t blockTime;
    const Block *curBlock;
    const Block *lastBlock;
    const Block *firstBlock;
    std::vector<Addr*> allAddrs;

    AllAddresses()
    {
        parser
        .usage("[options]")
        .version("")
        .description("dump the balance for all addresses that appear in the blockchain")
        .epilog("")
        ;
        parser
        .add_option("-a", "--atBlock")
        .action("store")
        .type("int")
        .set_default(-1)
        .help("only take into account transactions in blocks strictly older than <block> (default: all)")
        ;
        parser
        .add_option("-l", "--limit")
        .action("store")
        .type("int")
        .set_default(-1)
        .help("limit output to first N addresses, (default : output all addresses)")
        ;
    }

    virtual const char                   *name() const         {
        return "allAddresses";
    }
    virtual const optparse::OptionParser *optionParser() const {
        return &parser;
    }
    virtual bool                         needTXHash() const    {
        return true;
    }

    virtual void aliases(
        std::vector<const char*> &v
    ) const
    {
        v.push_back("addresses");
    }

    virtual int init(
        int argc,
        const char *argv[]
    )
    {
        curBlock = 0;
        lastBlock = 0;
        firstBlock = 0;

        addrMap.setEmptyKey(emptyKey);
        addrMap.resize(15 * 1000 * 1000);
        allAddrs.reserve(15 * 1000 * 1000);

        optparse::Values &values = parser.parse_args(argc, argv);
        cutoffBlock = values.get("atBlock");
        limit = values.get("limit");

        if(0<=cutoffBlock)
            info("Only taking into account transactions before block %" PRIu64 "\n", cutoffBlock);

        info("Analyzing blockchain...");
        return 0;
    }

    void move(
        const uint8_t *script,
        uint64_t      scriptSize,
        const uint8_t *txHash,
        int64_t        value,
        const uint8_t *downTXHash = 0
    )
    {
        uint8_t addrType[3];
        uint160_t pubKeyHash;
        int type = solveOutputScript(pubKeyHash.v, script, scriptSize, addrType);
        if(unlikely(type<0)) return;

        Addr *addr;
        auto i = addrMap.find(pubKeyHash.v);
        if(unlikely(addrMap.end()!=i))
            addr = i->second;
        else {
            addr = allocAddr();
            memcpy(addr->hash.v, pubKeyHash.v, kRIPEMD160ByteSize);
            addrMap[addr->hash.v] = addr;
            allAddrs.push_back(addr);
        }

        static uint64_t cnt = 0;
        if(unlikely(0==((cnt++)&0xFFFFF))) {

            if(
                curBlock   &&
                lastBlock  &&
                firstBlock
            )
            {
                double progress = curBlock->height/(double)lastBlock->height;
                info(
                    "%8" PRIu64 " blocks, "
                    "%8.3f MegaMoves , "
                    "%8.3f MegaAddrs , "
                    "%5.2f%%",
                    curBlock->height,
                    cnt*1e-6,
                    addrMap.size()*1e-6,
                    100.0*progress
                );
            }
        }
    }

    virtual void endOutput(
        const uint8_t *p,
        uint64_t      value,
        const uint8_t *txHash,
        uint64_t      outputIndex,
        const uint8_t *outputScript,
        uint64_t      outputScriptSize
    )
    {
        move(
            outputScript,
            outputScriptSize,
            txHash,
            value
        );
    }

    virtual void edge(
        uint64_t      value,
        const uint8_t *upTXHash,
        uint64_t      outputIndex,
        const uint8_t *outputScript,
        uint64_t      outputScriptSize,
        const uint8_t *downTXHash,
        uint64_t      inputIndex,
        const uint8_t *inputScript,
        uint64_t      inputScriptSize
    )
    {
        move(
            outputScript,
            outputScriptSize,
            upTXHash,
            -(int64_t)value,
            downTXHash
        );
    }

    virtual void wrapup()
    {
        info("done\n");

        auto e = allAddrs.end();
        auto s = allAddrs.begin();

        info("Dumping all addresses...");

        int64_t i = 0;
        while(likely(s<e)) {

            if(0<=limit && limit<=i)
                break;

            Addr *addr = *(s++);

            showHex(addr->hash.v, kRIPEMD160ByteSize, false);

            ++i;
        }
        
        info("done\n");

        info("found %" PRIu64 " addresses in total", (uint64_t)allAddrs.size());
        info("shown:%" PRIu64 " addresses", (uint64_t)i);
        printf("\n");
        exit(0);
    }

    virtual void start(
        const Block *s,
        const Block *e
    )
    {
        firstBlock = s;
        lastBlock = e;
    }

    virtual void startBlock(
        const Block *b,
        uint64_t chainSize
    )
    {
        curBlock = b;

        const uint8_t *p = b->data;
        SKIP(uint32_t, version, p);
        SKIP(uint256_t, prevBlkHash, p);
        SKIP(uint256_t, blkMerkleRoot, p);
        SKIP(uint32_t, bTime, p);

        if(0<=cutoffBlock && cutoffBlock<=curBlock->height)
            wrapup();
    }

};

static AllAddresses allAddresses;
