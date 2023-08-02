# SPDX-License-Identifier: WTFPL
# Copyright 2022-2023 rtldg <rtldg@protonmail.com>

import bz2
import csv
import glob
import hashlib
import mmap
import os
import shutil
from pathlib import Path
from stat import *
from datetime import datetime

csvname = "unprocessed/misc3.csv"
mapsfolder = "../todo-gb"
timestampFixer = False
skipExistingHash = False

if os.path.exists(csvname) and os.path.getsize(csvname) > 50:
    raise Exception("DONT OVERWRITE THAT CSV!")

def print_and_to_shit(s):
    print(s)
    with open("shit.txt", "a", encoding="utf-8") as shit:
        shit.write(s + "\n")

with open(csvname, "w", newline="", encoding="utf-8") as csvfile:
    mycsv = csv.writer(csvfile)
    mycsv.writerow(["mapname","filesize","filesize_bz2","sha1","note"])
    for filename in glob.iglob(mapsfolder + "/**/*.bsp", recursive=True):
        statttt = os.stat(filename)
        if S_ISDIR(statttt.st_mode):
            continue
        filesize = statttt.st_size
        if filesize == 0:
            print_and_to_shit(f"==== empty file {filename}")
            continue
        with open(filename, "rb") as f:
            mm = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
            vbspver = mm.read(5)
            if vbspver != b'VBSP\x13' and vbspver != b'VBSP\x14':
                thing = ""
                if vbspver == b'VBSP\x15':
                    thing = "==== skipping CS:GO map " + filename
                elif vbspver == b'VBSP\x19':
                    thing = "==== skipping Momentum Mod / Strata Source map " + filename
                elif vbspver[:4] == b'\x1E\x00\x00\x00' or vbspver[:4] == b'\x1D\x00\x00\x00':
                    thing = "==== skipping GoldSrc map? " + filename
                else:
                    thing = "==== not a CS:S map? " + filename
                print_and_to_shit(thing)
                continue
            mm.seek(0)
            digest = hashlib.sha1(mm).hexdigest()
            #print("Hash {} -- {}".format(digest, filename))
            renameto = "../hashed/" + digest + ".bsp"
            exists = os.path.exists(renameto)
            if not exists:
                """
                if timestampFixer:
                    print("wtf bad??? {} {}".format(filename, renameto))
                    mm.close()
                    continue
                """
                print("copying new! {} -> {}".format(filename, renameto))
                shutil.copy2(filename, renameto)
                with bz2.open(renameto + ".bz2", "wb") as fbz2:
                    mm.seek(0)
                    unused = fbz2.write(mm)
                shutil.copystat(renameto, renameto+".bz2")
            mm.close()
            if exists:
                if os.stat(renameto).st_size != filesize:
                    print_and_to_shit(f"??????? HASH COLLISION WOW {digest} {filename}")
                    continue
                mtimeThis = os.path.getmtime(filename)
                mtimeHashed = os.path.getmtime(renameto)
                if mtimeThis < mtimeHashed:
                    if timestampFixer:
                        print("timestamping {} from {} to {}".format(digest, datetime.utcfromtimestamp(mtimeHashed), datetime.utcfromtimestamp(mtimeThis)))
                        os.utime(renameto, (mtimeThis, mtimeThis))
                        os.utime(renameto+".bz2", (mtimeThis, mtimeThis))
                    else:
                        print("older timestamp for {} from {} -- {} -> {}!".format(digest, filename, datetime.utcfromtimestamp(mtimeHashed), datetime.utcfromtimestamp(mtimeThis)))
                continue
            if exists and skipExistingHash:
                continue
            filesize_bz2 = os.stat(renameto + ".bz2").st_size
            pp = Path(filename)
            mycsv.writerow([pp.stem,filesize,filesize_bz2,digest,str(pp.parent).replace("\\", "/").replace(mapsfolder+"/", "")])
