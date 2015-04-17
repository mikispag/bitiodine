#!/usr/bin/env python3

from sys import argv

import os, sys
lib_path = os.path.abspath('../common')
sys.path.append(lib_path)

from sqlite_wrapper import SQLiteWrapper
from queries import *
from util import *

import argparse
###

parser = argparse.ArgumentParser(description="BitIodine Cluster Labels: add labels to clusters.")
parser.add_argument('-d', dest='db', default="cluster_labels.sqlite",
           help='Cluster labels DB path')
parser.add_argument("--get", dest="get", default=None, help="Get label for a particular cluster ID")
parser.add_argument("--set", dest="set", nargs = 2, default=[], help="Set or replace the label for a particular cluster ID (--set <CLUSTER_ID> <LABEL>)")
options = parser.parse_args()

db = SQLiteWrapper(options.db)

if options.get is not None:
  try:
    label = db.query(get_cluster_label_query, (options.get,), fetch_one=True)
  except Exception as e:
    die('No label found for the cluster specified.')

  print(label)

elif len(options.set) > 1:
  try:
    res = db.query(add_cluster_label_query, (int(options.set[0]), options.set[1]))
  except Exception as e:
    die(e)

  print("Cluster {} now has label '{}'".format(int(options.set[0]), options.set[1]))

