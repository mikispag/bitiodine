#include <algorithm>
#include <arpa/inet.h>
#include <cstdint>
#include <cstring>
#include <errno.h>
#include <iostream>
#include <iterator>
#include <list>
#include <netdb.h>
#include <sqlite3.h>
#include <sstream>
#include <stdlib.h>
#include <sys/socket.h>
#include <thread>
#include <unistd.h>
#include <unordered_map>
#include <unordered_set>
#include <vector>
#include "csv.h"
#include <inttypes.h>

using namespace std;

// Allowed characters in a Bitcoin address
#define BITCOIN_CHARSET \
  "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"

struct ADDRESS; // forward declaration

struct TX {
  unsigned int            tx_id;
  string                  tx_hash;
  unsigned int            timestamp;
  unordered_set<ADDRESS *>inputs;
  vector<ADDRESS *>       recipients;
  vector<uint64_t>        amounts;
};

struct ADDRESS {
  string             address;
  unordered_set<TX *>txs_in;
  unordered_set<TX *>txs_out;
};

bool                 bitcoin_address_quick_valid(string address);
void                 do_command(char *command_c,
                                int   client);
string               find_path(string from,
                               string to);
unordered_set<string>find_predecessors(string from);
unordered_set<string>find_successors(string from);
void                 mainloop(int server_fd);
void                 print_cluster(int          client,
                                   unsigned int cluster);
void                 print_cluster_id(int    client,
                                      string address);
void                 print_cluster_label(int          client,
                                         unsigned int cluster);
int                  server_establish_connection(int server_fd);
void                 server_send(int    fd,
                                 string data);
int                  server_start_listen();
void                 server_thread(int rfd);

// Server constants
const char *PORT    = "8888"; // port numbers 1-1024 are probably reserved by
                              // your OS
const int   MAXLEN  = 1024;   // Max length of a message.
const int   MAXFD   = 128;    // Maximum file descriptors to use. Equals maximum
                              // clients.
const int   BACKLOG = 256;    // Number of connections that can wait in queue
                              // before they be accept()'ed

unordered_map<string, ADDRESS *>  addresses;
unordered_map<unsigned int, TX *> transactions;
unordered_map<ADDRESS *, unsigned int> clusters;

vector<string>tokenize(string const& input)
{
  istringstream str(input);

  istream_iterator<string> cur(str), end;
  return vector<string>(cur, end);
}

bool bitcoin_address_quick_valid(string address)
{
  const int length = address.length();

  if ((length < 27) || (length > 34)) {
    return false;
  }
  bool contains_invalid = address.find_first_not_of(BITCOIN_CHARSET) !=
                          string::npos;
  return !contains_invalid;
}

int main()
{
  sqlite3 *db;
  sqlite3_stmt *max_tx_id_stmt, *address_stmt, *details_stmt, *out_stmt;
  TX *tx_struct;
  ADDRESS *address_struct;
  ADDRESS *out_address_struct;
  unsigned int max_tx_id;

  if (sqlite3_open("../blockchain/blockchain.sqlite", &db) == SQLITE_OK) {
    // Prepare queries
    sqlite3_prepare_v2(db,
                       "SELECT MAX(tx_id) FROM tx",
                       -1,
                       &max_tx_id_stmt,
                       NULL);
    sqlite3_prepare_v2(db,
                       "SELECT txout.address FROM txin LEFT JOIN txout ON (txout.txout_id = txin.txout_id) WHERE txin.tx_id = ?",
                       -1,
                       &address_stmt,
                       NULL);
    sqlite3_prepare_v2(db,
                       "SELECT blocks.time, tx.tx_hash FROM tx LEFT JOIN blocks ON (tx.block_id = blocks.block_id) WHERE tx.tx_id = ?",
                       -1,
                       &details_stmt,
                       NULL);
    sqlite3_prepare_v2(db,
                       "SELECT txout.address, txout.txout_value FROM txout LEFT JOIN txin ON (txin.txout_id = txout.txout_id) WHERE txout.tx_id = ?",
                       -1,
                       &out_stmt,
                       NULL);

    if (sqlite3_step(max_tx_id_stmt) == SQLITE_ROW) {
      max_tx_id = (unsigned int)sqlite3_column_int64(max_tx_id_stmt, 0);
      cerr << "Number of transactions in the DB: " << max_tx_id << endl;
    } else {
      cerr << "Error: unable to get transactions." << endl;
      return 1;
    }

    sqlite3_finalize(max_tx_id_stmt);

    for (unsigned int tx_id = 1; tx_id <= max_tx_id; tx_id++) {
      if (tx_id % 1000000 == 0) {
        cerr << "Transaction: " << tx_id << endl;
      }

      sqlite3_reset(address_stmt);
      sqlite3_reset(details_stmt);
      sqlite3_reset(out_stmt);
      sqlite3_clear_bindings(address_stmt);
      sqlite3_clear_bindings(details_stmt);
      sqlite3_clear_bindings(out_stmt);

      // Bind parameters to compiled queries
      if (sqlite3_bind_int(address_stmt, 1, (int)tx_id) != SQLITE_OK) {
        cerr << "Error: unable to bind tx_id = " << tx_id <<
          " to address_stmt." << endl;
        return 1;
      }

      if (sqlite3_bind_int(details_stmt, 1, (int)tx_id) != SQLITE_OK) {
        cerr << "Error: unable to bind tx_id = " << tx_id <<
          " to details_stmt." << endl;
        return 1;
      }

      if (sqlite3_bind_int(out_stmt, 1, (int)tx_id) != SQLITE_OK) {
        cerr << "Error: unable to bind tx_id = " << tx_id << " to out_stmt." <<
          endl;
        return 1;
      }

      int address_status = sqlite3_step(address_stmt);

      // Skip "generated-only" transactions
      if (address_status == SQLITE_DONE) {
        continue;
      }

      tx_struct           = new TX();
      tx_struct->tx_id    = tx_id;
      transactions[tx_id] = tx_struct;

      while (address_status == SQLITE_ROW) {
        string address =
          string(reinterpret_cast<const char *>(sqlite3_column_text(address_stmt,
                                                                    0)));

        if (addresses.find(address) == addresses.end()) {
          address_struct          = new ADDRESS();
          address_struct->address = address;
          addresses[address]      = address_struct;
        } else {
          address_struct = addresses[address];
        }

        tx_struct->inputs.insert(address_struct);

        address_struct->txs_out.insert(tx_struct);

        if (sqlite3_step(details_stmt) == SQLITE_ROW) {
          unsigned int timestamp = (unsigned int)sqlite3_column_int64(
            details_stmt,
            0);
          string tx_hash         =
            string(reinterpret_cast<const char *>(sqlite3_column_text(details_stmt,
                                                                      1)));

          tx_struct->tx_hash   = tx_hash;
          tx_struct->timestamp = timestamp;
        }

        while (sqlite3_step(out_stmt) == SQLITE_ROW) {
          string out_address =
            string(reinterpret_cast<const char *>(sqlite3_column_text(out_stmt,
                                                                      0)));
          uint64_t amount = (uint64_t)sqlite3_column_int64(out_stmt, 1);

          if (addresses.find(out_address) == addresses.end()) {
            out_address_struct          = new ADDRESS();
            out_address_struct->address = out_address;
            addresses[out_address]      = out_address_struct;
          } else {
            out_address_struct = addresses[out_address];
          }

          tx_struct->recipients.push_back(out_address_struct);
          tx_struct->amounts.push_back(amount);

          out_address_struct->txs_in.insert(tx_struct);
        }

        address_status = sqlite3_step(address_stmt);
      }
    }
  } else {
    cerr << "Failed to open blockchain DB!" << endl;
  }

  sqlite3_finalize(address_stmt);
  sqlite3_finalize(details_stmt);
  sqlite3_finalize(out_stmt);
  sqlite3_close(db);

  cerr << "Loading clusters... ";

  io::CSVReader<2> in("../clusterizer/clusters.csv");
  in.read_header(io::ignore_extra_column, "address", "cluster");

  string address;
  unsigned int cluster;

  while (in.read_row(address, cluster)) {
    try {
      clusters[addresses.at(address)] = cluster;
    } catch (std::out_of_range& e) {
      // Do nothing.
    }
  }

  cerr << "done." << endl;

  int server_fd = server_start_listen();
  cerr << "Server started." << endl;

  if (server_fd == -1) {
    cerr << "An error occured. Closing program." << endl;
    return 1;
  }

  mainloop(server_fd);
  return 0;
}

void print_cluster_id(int client, string address)
{
  try {
    auto got = clusters.find(addresses.at(address));

    if (got == clusters.end()) {
      server_send(client, "500 Address not present in any cluster.\n");
      return;
    } else {
      unsigned int cluster = got->second;
      server_send(client, "BEGIN\n");
      server_send(client, to_string(cluster));
      server_send(client, "\nEND\n");
    }
  } catch (std::out_of_range& e) {
    server_send(client, "500 Address not in database.\n");
  }
}

void print_cluster_label(int client, unsigned int cluster)
{
  sqlite3 *db;
  sqlite3_stmt *stmt;

  if (sqlite3_open("../clusterizer/cluster_labels.sqlite", &db) == SQLITE_OK) {
    sqlite3_prepare_v2(db,
                       "SELECT label FROM cluster_labels WHERE cluster_id = ?",
                       -1,
                       &stmt,
                       NULL);

    if ((sqlite3_bind_int(stmt, 1,
                          cluster) != SQLITE_OK) ||
        (sqlite3_step(stmt) != SQLITE_ROW)) {
      server_send(client, "500 No label.\n");
      return;
    }
    string label =
      string(reinterpret_cast<const char *>(sqlite3_column_text(stmt, 0)));
    server_send(client, "BEGIN\n");
    server_send(client, label);
    server_send(client, "\nEND\n");
  }
  else {
    cerr << "Failed to open DB!" << endl;
    server_send(client, "500 No label.\n");
  }

  sqlite3_finalize(stmt);
  sqlite3_close(db);
}

void print_cluster(int client, unsigned int cluster)
{
  server_send(client, "BEGIN\n");

  for (auto& it : clusters) {
    if (it.second == cluster) {
      server_send(client, it.first->address + "\n");
    }
  }

  server_send(client, "END\n");
}

string find_path(string from, string to)
{
  // TODO: re-implement without Lemon
  return "";
}

unordered_set<string>find_successors(string from)
{
  unordered_set<string> successors;

  auto source_address                                  = new ADDRESS();
  unordered_map<string, ADDRESS *>::const_iterator got = addresses.find(from);

  if (got != addresses.end()) {
    source_address = got->second;

    for (auto& it : source_address->txs_out) {
      for (auto& recipient : it->recipients) {
        successors.insert(recipient->address);
      }
    }
  }

  return successors;
}

unordered_set<string>find_predecessors(string from)
{
  unordered_set<string> predecessors;

  auto source_address                                  = new ADDRESS();
  unordered_map<string, ADDRESS *>::const_iterator got = addresses.find(from);

  if (got != addresses.end()) {
    source_address = got->second;

    for (auto& it : source_address->txs_in) {
      for (auto& input : it->inputs) {
        predecessors.insert(input->address);
      }
    }
  }

  return predecessors;
}

unordered_set<string>a2a(string from, string to)
{
  unordered_set<string> tx_hashes;

  auto source_address                                  = new ADDRESS();
  unordered_map<string, ADDRESS *>::const_iterator got = addresses.find(from);

  if (got != addresses.end()) {
    source_address = got->second;

    for (auto& it : source_address->txs_out) {
      for (int i = 0; i != it->recipients.size(); i++) {
        if (it->recipients[i]->address == to) {
          stringstream output;
          output << it->tx_hash << "," << it->timestamp << "," << it->amounts[i];
          tx_hashes.insert(output.str());
        }
      }
    }
  }

  return tx_hashes;
}

unordered_set<string>a2c(string from, unsigned int cluster)
{
  unordered_set<string> tx_hashes;
  unordered_set<ADDRESS *> target_addresses;

  auto source_address = new ADDRESS();

  for (auto& it : clusters) {
    if (it.second == cluster) {
      target_addresses.insert(it.first);
    }
  }

  unordered_map<string, ADDRESS *>::const_iterator got = addresses.find(from);

  if (got != addresses.end()) {
    source_address = got->second;

    for (auto& it : source_address->txs_out) {
      for (int i = 0; i != it->recipients.size(); i++) {
        if (target_addresses.find(it->recipients[i]) != target_addresses.end()) {
          stringstream output;
          output << it->tx_hash << "," << it->timestamp << "," << it->amounts[i];
          tx_hashes.insert(output.str());
        }
      }
    }
  }

  return tx_hashes;
}

unordered_set<string>c2a(unsigned int cluster, string to)
{
  unordered_set<string> tx_hashes;
  unordered_set<ADDRESS *> source_addresses;

  auto dest_address = new ADDRESS();

  for (auto& it : clusters) {
    if (it.second == cluster) {
      source_addresses.insert(it.first);
    }
  }

  unordered_map<string, ADDRESS *>::const_iterator got = addresses.find(to);

  if (got != addresses.end()) {
    dest_address = got->second;

    for (auto& it : dest_address->txs_in) {
      for (auto& input : it->inputs) {
        if (source_addresses.find(input) != source_addresses.end()) {
          stringstream output;

          for (int i = 0; i < it->recipients.size(); i++) {
            if (it->recipients[i]->address == to) {
              output << it->tx_hash << "," << it->timestamp << "," <<
                it->amounts[i];
            }
          }
          tx_hashes.insert(output.str());
        }
      }
    }
  }

  return tx_hashes;
}

unordered_set<string>c2c(unsigned int cluster_from, unsigned int cluster_to)
{
  unordered_set<string> tx_hashes;
  unordered_set<ADDRESS *> source_addresses;
  unordered_set<ADDRESS *> target_addresses;

  for (auto& it : clusters) {
    if (it.second == cluster_from) {
      source_addresses.insert(it.first);
    }

    if (it.second == cluster_to) {
      target_addresses.insert(it.first);
    }
  }

  for (auto& source_address : source_addresses) {
    for (auto& it : source_address->txs_out) {
      for (int i = 0; i != it->recipients.size(); i++) {
        if (target_addresses.find(it->recipients[i]) != target_addresses.end()) {
          stringstream output;
          output << it->tx_hash << "," << it->timestamp << "," << it->amounts[i];
          tx_hashes.insert(output.str());
        }
      }
    }
  }

  return tx_hashes;
}

int server_start_listen()
{
  struct addrinfo hostinfo, *res;

  int sock_fd;

  int server_fd;
  int ret_bind = 1;

  memset(&hostinfo, 0, sizeof(hostinfo));

  hostinfo.ai_family   = AF_INET;
  hostinfo.ai_socktype = SOCK_STREAM;
  hostinfo.ai_flags    = AI_PASSIVE;

  getaddrinfo(NULL, PORT, &hostinfo, &res);

  server_fd = socket(res->ai_family, res->ai_socktype, res->ai_protocol);

  while (ret_bind != 0) {
    cerr << "Trying to kill already running process..." << endl;
    int ret_system = system("echo QUIT | nc -i 3 -q 5 127.0.0.1 8888");

    if (ret_system) {
      cerr << "system() returned " << ret_system << "." << endl;
    }

    ret_bind = ::bind(server_fd, res->ai_addr, res->ai_addrlen);
    cerr << "bind() returned: " << strerror(errno) << endl;
  }

  int ret_listen = listen(server_fd, BACKLOG);

  if (ret_listen != 0) {
    cerr << "Error on listen()." << endl;
    return -1;
  }

  return server_fd;
}

int server_establish_connection(int server_fd)
{
  char ipstr[INET_ADDRSTRLEN];
  int  port;
  int  new_fd;
  struct sockaddr_storage remote_info;
  socklen_t addr_size;

  addr_size = sizeof(addr_size);
  new_fd    = accept(server_fd, (struct sockaddr *)&remote_info, &addr_size);

  getpeername(new_fd, (struct sockaddr *)&remote_info, &addr_size);

  struct sockaddr_in *s = (struct sockaddr_in *)&remote_info;
  port = ntohs(s->sin_port);
  inet_ntop(AF_INET, &s->sin_addr, ipstr, sizeof ipstr);

  cerr << "Connection accepted from "  << ipstr <<  " using port " << port <<
    endl;
  return new_fd;
}

void server_send(int fd, string data)
{
  send(fd, data.c_str(), strlen(data.c_str()), 0);
}

void server_thread(int rfd)
{
  char buf[MAXLEN];

  // Send welcome message
  server_send(rfd, "200 BitIodine 1.0.0 READY\n");

  for (;;) {
    int buflen = read(rfd, buf, sizeof(buf));

    if (buflen <= 0) {
      cerr << "Client disconnected. Clearing fd " << rfd << endl;
      close(rfd);
      return;
    }

    do_command(buf, rfd);
  }
}

void mainloop(int server_fd)
{
  for (;;) {
    int rfd = server_establish_connection(server_fd);

    if (rfd >= 0) {
      cerr << "Client connected. Using file descriptor " << rfd << endl;

      if (rfd > MAXFD) {
        cerr << "Too many clients are trying to connect." << endl;
        close(rfd);
        continue;
      }

      // Spawn a new thread and detach it immediately
      thread(server_thread, rfd).detach();
    }
  }
}

void do_command(char *command_c, int client)
{
  string command = command_c;
  string output;

  vector<string> tokens;
  try {
    tokens = tokenize(command);
  } catch (...) {
    server_send(client, "404 COMMAND NOT FOUND\n");
    return;
  }

  if (tokens[0] == "SHORTEST_PATH_A2A") {
    if ((tokens.size() < 3) || !bitcoin_address_quick_valid(tokens[1]) ||
        !bitcoin_address_quick_valid(tokens[2])) {
      server_send(client, "500 Arguments error.\n");
      return;
    }

    output = find_path(tokens[1], tokens[2]);

    if (!output.empty()) {
      server_send(client, "BEGIN\n");
      server_send(client, output);
      server_send(client, "END\n");
    } else {
      server_send(client, "500 No path.\n");
    }
    return;
  } else if (tokens[0] == "SUCCESSORS") {
    if (tokens.size() < 2) {
      server_send(client, "500 Arguments error.\n");
      return;
    }

    server_send(client, "BEGIN\n");

    unordered_set<string> successors = find_successors(tokens[1]);

    if (!successors.empty()) {
      for (auto& it : successors) {
        output += it + ",";
      }

      /* Remove last character (,) */
      try {
        output.pop_back();
      } catch (...) {
        // Do nothing for now.
      }
      server_send(client, output + "\n");
      server_send(client, "END\n");
    } else  server_send(client, "500 No successors.\n");
    return;
  } else if (tokens[0] == "PREDECESSORS") {
    if ((tokens.size() < 2) || !bitcoin_address_quick_valid(tokens[1])) {
      server_send(client, "500 Arguments error.\n");
      return;
    }

    server_send(client, "BEGIN\n");

    unordered_set<string> predecessors = find_predecessors(tokens[1]);

    if (!predecessors.empty()) {
      for (auto& it : predecessors) {
        output += it + ",";
      }

      /* Remove last character (,) */
      try {
        output.pop_back();
      } catch (...) {
        // Do nothing for now.
      }
      server_send(client, output + "\n");
      server_send(client, "END\n");
    } else  server_send(client, "500 No predecessors.\n");
    return;
  } else if (tokens[0] == "A2A") {
    if ((tokens.size() < 3) || !bitcoin_address_quick_valid(tokens[1]) ||
        !bitcoin_address_quick_valid(tokens[2])) {
      server_send(client, "500 Arguments error.\n");
      return;
    }

    server_send(client, "BEGIN\n");

    unordered_set<string> tx_hashes = a2a(tokens[1], tokens[2]);

    if (!tx_hashes.empty()) {
      for (auto& it : tx_hashes) {
        server_send(client, it + "\n");
      }

      server_send(client, "END\n");
    } else  server_send(client, "500 No transactions.\n");
    return;
  } else if (tokens[0] == "A2C") {
    if ((tokens.size() < 3) || !bitcoin_address_quick_valid(tokens[1])) {
      server_send(client, "500 Arguments error.\n");
      return;
    }

    server_send(client, "BEGIN\n");

    unordered_set<string> tx_hashes =
      a2c(tokens[1], (unsigned int)stoi(tokens[2]));

    if (!tx_hashes.empty()) {
      for (auto& it : tx_hashes) {
        server_send(client, it + "\n");
      }
    } else {
      server_send(client, "500 No transactions.\n");
    }

    server_send(client, "END\n");
    return;
  } else if (tokens[0] == "C2A") {
    if ((tokens.size() < 3) || !bitcoin_address_quick_valid(tokens[2])) {
      server_send(client, "500 Arguments error.\n");
      return;
    }

    server_send(client, "BEGIN\n");

    unordered_set<string> tx_hashes =
      c2a((unsigned int)stoi(tokens[1]), tokens[2]);

    if (!tx_hashes.empty()) {
      for (auto& it : tx_hashes) {
        server_send(client, it + "\n");
      }
    } else {
      server_send(client, "500 No transactions.\n");
    }

    server_send(client, "END\n");
    return;
  } else if (tokens[0] == "C2C") {
    if (tokens.size() < 3) {
      server_send(client, "500 Arguments error.\n");
      return;
    }

    server_send(client, "BEGIN\n");

    unordered_set<string> tx_hashes =
      c2c((unsigned int)stoi(tokens[1]), (unsigned int)stoi(tokens[2]));

    if (!tx_hashes.empty()) {
      for (auto& it : tx_hashes) {
        server_send(client, it + "\n");
      }
    } else {
      server_send(client, "500 No transactions.\n");
    }

    server_send(client, "END\n");
    return;
  } else if (tokens[0] == "PRINT_CLUSTER") {
    unsigned int cluster;

    if (tokens.size() < 2) {
      server_send(client, "500 Arguments error.\n");
      return;
    }

    try {
      print_cluster(client, stoi(tokens[1]));
    } catch (std::invalid_argument e) {
      server_send(client, "500 Arguments error.\n");
    }
    return;
  } else if (tokens[0] == "PRINT_CLUSTER_ID") {
    if (tokens.size() < 2) {
      server_send(client, "500 Arguments error.\n");
      return;
    }

    try {
      print_cluster_id(client, tokens[1]);
    } catch (std::invalid_argument e) {
      server_send(client, "500 Arguments error.\n");
    }
    return;
  } else if (tokens[0] == "PRINT_CLUSTER_LABEL") {
    unsigned int cluster;

    if (tokens.size() < 2) {
      server_send(client, "500 Arguments error.\n");
      return;
    }

    try {
      print_cluster_label(client, stoi(tokens[1]));
    } catch (std::invalid_argument e) {
      server_send(client, "500 Arguments error.\n");
    }
    return;
  } else if (tokens[0] == "PRINT_NEIGHBORS") {
    unsigned int cluster;

    if ((tokens.size() < 2) || !bitcoin_address_quick_valid(tokens[1])) {
      server_send(client, "500 Arguments error.\n");
      return;
    }

    try {
      auto got = clusters.find(addresses.at(tokens[1]));

      if (got == clusters.end()) {
        server_send(client, "500 Address not present in any cluster.\n");
        return;
      } else {
        cluster = got->second;
      }

      print_cluster(client, cluster);
    } catch (std::out_of_range& e) {
      server_send(client, "500 Address not present in database.\n");
    }
    return;
  } else if (tokens[0] == "STATS") {
    server_send(client, "BEGIN\n");
    server_send(client, to_string(addresses.size()));
    server_send(client, "\n");
    server_send(client, to_string(transactions.size()));
    server_send(client, "\nEND\n");

    return;
  } else if (tokens[0] == "QUIT") {
    exit(0);
  } else {
    server_send(client, "404 COMMAND NOT FOUND\n");
  }
}
