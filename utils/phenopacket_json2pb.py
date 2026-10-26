#! /usr/bin/env python3
# /// script
# dependencies = [
#   "phenopackets",
#   "protobuf"
# ]
# ///

USAGE = """usage: uv run phenopacket_json2pb.py [path to phenopacket.json]
This script reads a given phenopacket as json and writes it in binary format as a protobuf file.
It is intended to be used for updating the phenopackets.pb file, incorporating changed made to the phenopackets.json file.

For an input file "FOO.json" it will write the binary version to "FOO.pb" and overwrite, if said file already exists.

author = Niclas Grote
"""
from sys import argv
from pathlib import Path

from google.protobuf import json_format
from phenopackets import Phenopacket

def read_phenopacket_json(path):
    with open(path, 'r') as jsfile:
        phenopacket = json_format.Parse(message=Phenopacket(), text=jsfile.read())
        return phenopacket

def write_phenopacket_pb(phenopacket, path):
    binary_string = phenopacket.SerializeToString()
    with open(path, 'bw') as pbfile:
        pbfile.write(binary_string)

if __name__ == "__main__":
    if len(argv) != 2:
        print("please provide the path to the file you want to convert to protobuf.")
        print(USAGE)
        exit(1)

    infile = Path(argv[1])
    outfile = infile.with_suffix(".pb")
    print(f"converting {infile} -> {outfile}")

    phenopacket = read_phenopacket_json(infile)
    write_phenopacket_pb(phenopacket, outfile)
    print("done.")
