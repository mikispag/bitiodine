#include <fstream>
#include <util.h>
#include <stdio.h>
#include <common.h>
#include <errlog.h>
#include <option.h>
#include <callback.h>
#include "sqlite3pp.h"

static uint8_t empty[kSHA256ByteSize] = { 0x42 };

typedef GoogMap<
    Hash256,
    uint64_t,
    Hash256Hasher,
    Hash256Equal
    >::Map OutputMap;

inline bool file_exists(const std::string& name)
{
  std::ifstream file(name);

  return file.good();
}

static void writeHexEscapedBinaryBuffer(
  FILE          *f,
  const uint8_t *p,
  size_t         n
  )
{
  char buf[3];

  p += n;

  while (n--)
  {
    uint8_t c = *(--p);
    sprintf(buf, "%02X", (unsigned char)c);
    fputs(buf, f);
  }
}

struct SQLDump : public Callback {
  FILE *txFile;
  FILE *blockFile;
  FILE *inputFile;
  FILE *outputFile;

  uint64_t               txID;
  uint64_t               blkID;
  uint64_t               inputID;
  uint64_t               outputID;
  uint64_t               firstBlock;
  uint64_t               lastBlock;
  OutputMap              outputMap;
  optparse::OptionParser parser;

  SQLDump() {
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
    .set_default(-1)
    .help("stop dump at block <block> (default: all)")
    ;
  }

  virtual const char* name() const {
    return "sqldump";
  }

  virtual const optparse::OptionParser* optionParser() const {
    return &parser;
  }

  virtual bool needUpstream() const {
    return true;
  }

  virtual void aliases(
    std::vector<const char *>& v
    ) const {
    v.push_back("dump");
  }

  virtual int init(
    int         argc,
    const char *argv[]
    ) {
    txID     = 0;
    blkID    = 0;
    inputID  = 0;
    outputID = 0;

    static uint64_t sz = 32 * 1000 * 1000;
    outputMap.setEmptyKey(empty);
    outputMap.resize(sz);

    optparse::Values& values = parser.parse_args(argc, argv);

    if (values.is_set_by_user("atBlock")) {
      lastBlock = values.get("atBlock").asUInt64();
      info("Last block: %" PRIu64 ".\n", lastBlock);
    } else {
      lastBlock = 0;
    }

    if (!file_exists("../blockchain/blockchain.sqlite"))
    {
      firstBlock = 0;
    }
    else
    {
      sqlite3pp::database db("../blockchain/blockchain.sqlite");
      sqlite3pp::query    qry(db, "SELECT MAX(block_id) FROM blocks");

      sqlite3pp::query::iterator i = qry.begin();

      firstBlock = (uint64_t)atoi((*i).get<char const *>(0)) + 1;

      info("Resuming from block %" PRIu64 ".\n", firstBlock);
    }

    info("Dumping the blockchain...");

    txFile = fopen("tx.txt", "w");

    if (!txFile) sysErrFatal("couldn't open file txs.txt for writing\n");

    blockFile = fopen("blocks.txt", "w");

    if (!blockFile) sysErrFatal("couldn't open file blocks.txt for writing\n");

    inputFile = fopen("txin.txt", "w");

    if (!inputFile) sysErrFatal("couldn't open file inputs.txt for writing\n");

    outputFile = fopen("txout.txt", "w");

    if (!outputFile) sysErrFatal("couldn't open file outputs.txt for writing\n");

    FILE *sqlFile = fopen("blockchain.sql", "w");

    if (!sqlFile) sysErrFatal("couldn't open file blockchain.sql for writing\n");

    fprintf(
      sqlFile,
      "PRAGMA journal_mode=MEMORY;\n"
      "PRAGMA synchronous=NORMAL;\n"
      "CREATE TABLE IF NOT EXISTS blocks(\n"
      "    block_id BIGINT NOT NULL PRIMARY KEY,\n"
      "    block_hash TEXT NOT NULL,\n"
      "    time BIGINT NOT NULL\n"
      ");\n"
      "CREATE TABLE IF NOT EXISTS tx(\n"
      "    tx_id BIGINT NOT NULL PRIMARY KEY,\n"
      "    tx_hash TEXT NOT NULL,\n"
      "    block_id BIGINT NOT NULL,\n"
      "    FOREIGN KEY (block_id) REFERENCES blocks (block_id)\n"
      ");\n"
      "CREATE TABLE IF NOT EXISTS txout(\n"
      "    txout_id BIGINT NOT NULL PRIMARY KEY,\n"
      "    address CHAR(40),\n"
      "    txout_value BIGINT NOT NULL,\n"
      "    tx_id BIGINT NOT NULL,\n"
      "    txout_pos INT NOT NULL,\n"
      "    FOREIGN KEY (tx_id) REFERENCES tx (tx_id)\n"
      ");\n"
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
      "CREATE INDEX IF NOT EXISTS x_txout_value ON txout (txout_value);\n"
      "CREATE VIEW IF NOT EXISTS tx_full AS SELECT blocks.time, tx.tx_hash, tx.tx_id, txout.address, txout.txout_value FROM txout LEFT JOIN tx ON (tx.tx_id = txout.tx_id) LEFT JOIN blocks ON (tx.block_id = blocks.block_id);\n"
      "\n"
      );
    fclose(sqlFile);

    FILE *bashFile = fopen("blockchain.sh", "w");

    if (!bashFile) sysErrFatal("Couldn't open file blockchain.sh for writing!\n");


    fprintf(
      bashFile,
      "echo 'Recreating DB blockchain...'\n"
      "mkdir ../blockchain\n"
      "sqlite3 ../blockchain/blockchain.sqlite < blockchain.sql\n"
      "rm -f blockchain.sql\n"
      "echo done.\n"
      "for i in blocks txin txout tx\n"
      "do\n"
      "    echo \"Importing table ${i}...\"\n"
      "    echo \".import $i.txt $i\" | sqlite3 ../blockchain/blockchain.sqlite\n"
      "    echo done.\n"
      "    rm -f $i.txt\n"
      "    echo\n"
      "done\n"
      "rm -f blockchain.sh\n"
      "\n"
      );
    fclose(bashFile);

    return 0;
  }

  virtual void startBlock(
    const Block *b,
    uint64_t
    ) {
    if (lastBlock > 0 && lastBlock < static_cast<uint64_t>(b->height)) {
      wrapup();
    }

    auto p = b->chunk->getData();
    uint8_t blockHash[kSHA256ByteSize];
    sha256Twice(blockHash, p, 80);

    SKIP(uint32_t,  version,       p);
    SKIP(uint256_t, prevBlkHash,   p);
    SKIP(uint256_t, blkMerkleRoot, p);
    LOAD(uint32_t, blkTime, p);

    blkID = b->height - 1;

    if (blkID >= firstBlock) {
      fprintf(blockFile, "%" PRIu64 "|", blkID);

      writeHexEscapedBinaryBuffer(blockFile, blockHash, kSHA256ByteSize);
      fputc('|', blockFile);
      fprintf(blockFile, "%" PRIu64 "\n", (uint64_t)blkTime);
    }

    if (0 == blkID % 5000) {
      fprintf(
        stderr,
        "block=%8" PRIu64 " "
        "nbOutputs=%9" PRIu64 "\n",
        blkID,
        outputMap.size()
        );
    }
  }

  virtual void startTX(
    const uint8_t *p,
    const uint8_t *hash
    ) {
    if (blkID >= firstBlock) {
      fprintf(txFile, "%" PRIu64 "|", txID);

      writeHexEscapedBinaryBuffer(txFile, hash, kSHA256ByteSize);
      fputc('|', txFile);

      fprintf(txFile, "%" PRIu64 "\n", blkID);
    }
    txID++;
  }

  virtual void endOutput(
    const uint8_t *p,
    uint64_t       value,
    const uint8_t *txHash,
    uint64_t       outputIndex,
    const uint8_t *outputScript,
    uint64_t       outputScriptSize
    ) {
    uint8_t address[40];

    address[0] = 'X';
    address[1] = 0;

    uint8_t   addrType[3];
    uint160_t pubKeyHash;
    int type = solveOutputScript(
      pubKeyHash.v,
      outputScript,
      outputScriptSize,
      addrType
      );

    if (likely(0 <= type)) {
      hash160ToAddr(
        address,
        pubKeyHash.v,
        false,
        addrType[0]
        );
    }

    if (blkID >= firstBlock) {
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
    }

    uint32_t oi = outputIndex;
    uint8_t *h  = allocHash256();
    memcpy(h, txHash, kSHA256ByteSize);

    uintptr_t ih  = reinterpret_cast<uintptr_t>(h);
    uint32_t *h32 = reinterpret_cast<uint32_t *>(ih);
    h32[0] ^= oi;

    outputMap[h] = outputID++;
  }

  virtual void edge(
    uint64_t       value,
    const uint8_t *upTXHash,
    uint64_t       outputIndex,
    const uint8_t *outputScript,
    uint64_t       outputScriptSize,
    const uint8_t *downTXHash,
    uint64_t       inputIndex,
    const uint8_t *inputScript,
    uint64_t       inputScriptSize
    ) {
    uint256_t h;
    uint32_t  oi = outputIndex;

    memcpy(h.v, upTXHash, kSHA256ByteSize);

    uintptr_t ih  = reinterpret_cast<uintptr_t>(h.v);
    uint32_t *h32 = reinterpret_cast<uint32_t *>(ih);
    h32[0] ^= oi;

    auto src = outputMap.find(h.v);

    if (outputMap.end() == src) {
      errFatal("Unconnected input.");
    }

    if (blkID >= firstBlock) {
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
        (uint32_t)inputIndex
        );
    } else {
      inputID++;
    }
  }

  virtual void wrapup() {
    fclose(outputFile);
    fclose(inputFile);
    fclose(blockFile);
    fclose(txFile);
    system("/bin/bash ./blockchain.sh");
    info("Done.");
    exit(0);
  }
};

static SQLDump sqlDump;
