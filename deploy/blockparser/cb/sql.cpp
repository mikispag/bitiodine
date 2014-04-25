
// Full SQL dump of the blockchain

#include <fstream>

#include <util.h>
#include <stdio.h>
#include <stdlib.h>
#include <common.h>
#include <errlog.h>
#include <option.h>
#include <callback.h>
#include <unistd.h>
#include <stdint.h>

static uint8_t empty[kSHA256ByteSize] = { 0x42 };
typedef GoogMap<Hash256, uint64_t, Hash256Hasher, Hash256Equal>::Map OutputMap;

inline bool file_exists (const std::string &name)
{
    std::ifstream file(name);
    return (file.good());
}

static void writeEscapedBinaryBuffer(
    FILE          *f,
    const uint8_t *p,
    size_t        n
)
{
    char buf[3];
    p += n;

    while (n--)
    {
        uint8_t c = *(--p);
        sprintf(buf, "%02X", (unsigned char) c);
        fputs(buf, f);
    }
}

struct SQLDump: public Callback
{
    FILE *txFile;
    FILE *bashFile;
    FILE *blockFile;
    FILE *inputFile;
    FILE *outputFile;
    FILE *progressFile;

    uint64_t txID;
    uint64_t blkID;
    uint64_t inputID;
    uint64_t outputID;
    uint64_t cutoffBlock;
    OutputMap outputMap;
    optparse::OptionParser parser;

    SQLDump()
    {
        parser
        .usage("[options]")
        .version("")
        .description("create an SQL dump of the blockchain")
        .epilog("")
        ;
        parser
        .add_option("-a", "--atBlock")
        .action("store")
        .type("int")
        .set_default(1)
        .help("start dump at block <block> (default: genesis block) - this is ignored if progress.txt exists in the current path")
        ;
    }

    virtual const char                   *name() const
    {
        return "sqldump";
    }
    virtual const optparse::OptionParser *optionParser() const
    {
        return &parser;
    }
    virtual bool                         needTXHash() const
    {
        return true;
    }

    virtual void aliases(
        std::vector<const char *> &v
    ) const
    {
        v.push_back("dump");
    }

    virtual int init(
        int argc,
        const char *argv[]
    )
    {
        txID = 0;
        blkID = 0;
        inputID = 0;
        outputID = 0;

        static uint64_t sz = 32 * 1000 * 1000;
        outputMap.setEmptyKey(empty);
        outputMap.resize(sz);

        optparse::Values &values = parser.parse_args(argc, argv);
        if (!file_exists("progress.txt"))
        {
            cutoffBlock = values.get("atBlock");
        }
        else
        {
            progressFile = fopen("progress.txt", "r");
            if (fscanf(progressFile, "%" PRIu64, &cutoffBlock) != 1) {
                sysErrFatal("Couldn't get progress information from progress.txt\n");
            }
            fclose(progressFile);
            cutoffBlock++;
        }

        info("Dumping the blockchain...");

        txFile = fopen("tx.txt", "w");
        if (!txFile) sysErrFatal("Couldn't open file tx.txt for writing\n");

        blockFile = fopen("blocks.txt", "w");
        if (!blockFile) sysErrFatal("Couldn't open file blocks.txt for writing\n");

        inputFile = fopen("txin.txt", "w");
        if (!inputFile) sysErrFatal("Couldn't open file txin.txt for writing\n");

        outputFile = fopen("txout.txt", "w");
        if (!outputFile) sysErrFatal("Couldn't open file txout.txt for writing\n");

        FILE *sqlFile = fopen("blockchain.sql", "w");
        if (!sqlFile) sysErrFatal("Couldn't open file blockchain.sql for writing\n");

        fprintf(
            sqlFile,
            "PRAGMA journal_mode=memory;\n"
            "PRAGMA synchronous=0;\n"
            "CREATE TABLE IF NOT EXISTS blocks(\n"
            "    block_id BIGINT NOT NULL PRIMARY KEY,\n"
            "    block_hash TEXT NOT NULL,\n"
            "    time BIGINT NOT NULL\n"
            ");\n"
            "\n"
            "CREATE TABLE IF NOT EXISTS tx(\n"
            "    tx_id BIGINT NOT NULL PRIMARY KEY,\n"
            "    tx_hash TEXT NOT NULL,\n"
            "    block_id BIGINT NOT NULL,\n"
            "    FOREIGN KEY (block_id) REFERENCES blocks (block_id)\n"
            ");\n"
            "\n"
            "CREATE TABLE IF NOT EXISTS txout(\n"
            "    txout_id BIGINT NOT NULL PRIMARY KEY,\n"
            "    address CHAR(40),\n"
            "    txout_value BIGINT NOT NULL,\n"
            "    tx_id BIGINT NOT NULL,\n"
            "    txout_pos INT NOT NULL,\n"
            "    FOREIGN KEY (tx_id) REFERENCES tx (tx_id)\n"
            ");\n"
            "\n"
            "CREATE TABLE IF NOT EXISTS txin(\n"
            "    txin_id BIGINT NOT NULL PRIMARY KEY,\n"
            "    txout_id BIGINT NOT NULL,\n"
            "    tx_id BIGINT NOT NULL,\n"
            "    txin_pos INT NOT NULL,\n"
            "    FOREIGN KEY (tx_id) REFERENCES tx (tx_id)\n"
            ");\n"
            "CREATE INDEX IF NOT EXISTS x_txin_txout ON txin (txout_id);\n"
            "CREATE INDEX IF NOT EXISTS x_txout_address ON txout (address);\n"
            "CREATE INDEX IF NOT EXISTS x_txin_txid ON txin (tx_id);\n"
            "CREATE INDEX IF NOT EXISTS x_txout_txid ON txout (tx_id);\n"
            "CREATE INDEX IF NOT EXISTS x_tx_txid ON tx (tx_id);\n"
            "CREATE INDEX IF NOT EXISTS x_txout_value ON txout(txout_value);\n"
            "CREATE INDEX IF NOT EXISTS x_blocks_id ON blocks(block_id);\n"
            "CREATE VIEW IF NOT EXISTS tx_full AS SELECT blocks.time, tx.tx_hash, tx.tx_id, txout.address, txout.txout_value FROM txout LEFT JOIN tx ON (tx.tx_id = txout.tx_id) LEFT JOIN blocks ON (tx.block_id = blocks.block_id);\n"
            "\n"
        );
        fclose(sqlFile);

        bashFile = fopen("blockchain.sh", "w");
        if (!bashFile) sysErrFatal("Couldn't open file blockchain.sh for writing!\n");

        // EDIT here: using /run/shm/ for performance on a server - modify to local path!
        fprintf(
            bashFile,
            "\n"
            "#!/bin/bash\n"
            "\n"
            "echo 'Recreating DB blockchain...'\n"
            "rm -f /run/shm/blockchain.sqlite\n"
            "cp ../blockchain/blockchain.sqlite /run/shm/\n"
            "mkdir ../blockchain\n"
            "sqlite3 /run/shm/blockchain.sqlite < blockchain.sql\n"
            "echo done\n"
            "echo\n"
            "rm -f blockchain.sql\n"
            "\n"
            "for i in blocks tx txin txout\n"
            "do\n"
            "    echo Importing table $i ...\n"
            "    echo \".import $i.txt $i\" | sqlite3 /run/shm/blockchain.sqlite\n"
            "    echo done\n"
            "    rm -f $i.txt\n"
            "    echo\n"
            "done\n"
            "mv -f /run/shm/blockchain.sqlite ../blockchain/blockchain.sqlite"
            "rm -f blockchain.sh\n"
            "\n"
        );
        fclose(bashFile);

        return 0;
    }

    virtual void startBlock(
        const Block *b,
        uint64_t chainSize
    )
    {
        if (cutoffBlock > 0 && (uint64_t)b->height < cutoffBlock)
        {
            return;
        }

        uint8_t blockHash[kSHA256ByteSize];
        sha256Twice(blockHash, b->data, 80);

        const uint8_t *p = b->data;
        SKIP(uint32_t, version, p);
        SKIP(uint256_t, prevBlkHash, p);
        SKIP(uint256_t, blkMerkleRoot, p);
        LOAD(uint32_t, blkTime, p);

        // block_id BIGINT PRIMARY KEY
        // block_hash BINARY(32)
        // time BIGINT
        fprintf(blockFile, "%" PRIu64 "|", (blkID = b->height - 1));

        writeEscapedBinaryBuffer(blockFile, blockHash, kSHA256ByteSize);
        fputc('|', blockFile);

        fprintf(blockFile, "%" PRIu64 "\n", (uint64_t)blkTime);
        if (0 == (b->height) % 5000)
        {
            fprintf(
                stderr,
                "block=%8" PRIu64 " "
                "nbOutputs=%8" PRIu64 "\n",
                b->height,
                outputMap.size()
            );
        }
    }

    virtual void startTX(
        const uint8_t *p,
        const uint8_t *hash
    )
    {
        if (cutoffBlock > 0 && blkID + 1 < cutoffBlock)
        {
            return;
        }

        // tx_id BIGINT PRIMARY KEY
        // tx_hash BINARY(32)
        // block_id BIGINT
        fprintf(txFile, "%" PRIu64 "|", ++txID);

        writeEscapedBinaryBuffer(txFile, hash, kSHA256ByteSize);
        fputc('|', txFile);

        fprintf(txFile, "%" PRIu64 "\n", blkID);
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
        if (cutoffBlock > 0 && blkID + 1 < cutoffBlock)
        {
            return;
        }

        uint8_t address[40];
        address[0] = 'X';
        address[1] = 0;

        uint8_t addrType[3];
        uint160_t pubKeyHash;
        int type = solveOutputScript(pubKeyHash.v, outputScript, outputScriptSize, addrType);
        if (likely(0 <= type)) hash160ToAddr(address, pubKeyHash.v);

        // txout_id BIGINT PRIMARY KEY
        // address CHAR(40)
        // txout_value BIGINT
        // tx_id BIGINT
        // txout_pos INT
        fprintf(
            outputFile,
            "%" PRIu64 "|"
            "%s|"
            "%" PRIu64 "|"
            "%" PRIu64 "|"
            "%" PRIu32 "\n"
            ,
            outputID,
            address,
            value,
            txID,
            (uint32_t)outputIndex
        );

        uint32_t oi = outputIndex;
        uint8_t *h = allocHash256();
        memcpy(h, txHash, kSHA256ByteSize);

        uintptr_t ih = reinterpret_cast<uintptr_t>(h);
        uint32_t *h32 = reinterpret_cast<uint32_t *>(ih);
        h32[0] ^= oi;

        outputMap[h] = outputID++;
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
        if (cutoffBlock > 0 && blkID + 1 < cutoffBlock)
        {
            return;
        }

        uint256_t h;
        uint32_t oi = outputIndex;
        memcpy(h.v, upTXHash, kSHA256ByteSize);

        uintptr_t ih = reinterpret_cast<uintptr_t>(h.v);
        uint32_t *h32 = reinterpret_cast<uint32_t *>(ih);
        h32[0] ^= oi;

        auto src = outputMap.find(h.v);
        if (outputMap.end() == src) errFatal("Unconnected input!");

        // ...
        fprintf(
            inputFile,
            "%" PRIu64 "|"
            "%" PRIu64 "|"
            "%" PRIu64 "|"
            "%" PRIu32 "\n"
            ,
            inputID++,
            src->second,
            txID,
            (uint32_t)outputIndex
        );
    }

    virtual void wrapup()
    {
        fclose(outputFile);
        fclose(inputFile);
        fclose(blockFile);
        fclose(txFile);
        system("chmod +x ./blockchain.sh");
        system("./blockchain.sh");
        info("Done.\n");
        progressFile = fopen("progress.txt", "w");
        fprintf(progressFile, "%" PRIu64, blkID + 1);
        fclose(progressFile);
        info("Wrote progress file.\n");
        exit(0);
    }
};

static SQLDump sqlDump;
