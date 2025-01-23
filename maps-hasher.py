# SPDX-License-Identifier: WTFPL
# Copyright 2022-2024 rtldg <rtldg@protonmail.com>

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

def print_and_to_shit(s):
    print(s)
    with open("shit.txt", "a", encoding="utf-8") as shit:
        shit.write(s + "\n")

def normal_name(m):
    return m.strip().replace('.', '_').lower()

def main(csvname, automatic, mapsfolder, timestampFixer, skipExistingHash, canonClobberCheck):
    if not automatic and os.path.exists(csvname) and os.path.getsize(csvname) > 50:
        raise Exception("DONT OVERWRITE THAT CSV!")

    existing_canon = {}
    if canonClobberCheck:
        canonClobber = open("canonClobber.csv", "a", encoding="utf-8")
        with open("processed/main.fastdl.me/maps_index.html.csv", newline='', encoding="utf-8") as f:
            for line in csv.reader(f):
                existing_canon[normal_name(line[0])] = line[1]

    existing_names = {}
    existing_recents = {}
    if automatic:
        # this will fail if you haven't ran `python process.py` yet
        with open("processed/main.fastdl.me/hashed_index.html.csv", newline='', encoding="utf-8") as f:
            for line in csv.reader(f):
                existing_names[normal_name(line[0])] = line[4] # url, hopefully from gb
        # only allow clobbering existing map names for recently added gamebanana downloads...
        with open("recently_added.csv", newline='', encoding="utf-8") as f:
            for line in csv.reader(f):
                if line[0] == "mapname":
                    continue
                existing_recents[line[0]] = line[4].split("_")[0]
                if line[0] in existing_names:
                    # lolololololol... this entire site is such a hack...
                    if ("https://gamebanana.com/mods/"+line[4].split("_")[0]) == existing_names[line[0]]:
                        del existing_names[line[0]]

    newly_hashed = []
    with open(csvname, ("a" if automatic else "w"), newline="", encoding="utf-8") as csvfile:
        mycsv = csv.writer(csvfile)
        if not automatic:
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
                            continue
                        else:
                            print("older timestamp for {} from {} -- {} -> {}!".format(digest, filename, datetime.utcfromtimestamp(mtimeHashed), datetime.utcfromtimestamp(mtimeThis)))
                if exists and skipExistingHash:
                    continue
                filesize_bz2 = os.stat(renameto + ".bz2").st_size
                pp = Path(filename)
                stem = pp.stem
                if stem.startswith("#") and len(stem) > 1:
                    stem = stem[1:]
                row = [stem,filesize,filesize_bz2,digest,str(pp.parent).replace("\\", "/").replace(mapsfolder+"/", "")]
                if automatic and normal_name(row[0]) in existing_names:
                    if existing_recents.get(normal_name(row[0]), "mrbeast") != row[4].split("_")[0]:
                        row[0] = "#" + row[0]
                if canonClobberCheck and not automatic:
                    if row[0] in existing_canon and existing_canon[row[0]] != row[3]:
                        canonClobber.write(f"{row[0]},{existing_canon[row[0]]},\n")
                        print_and_to_shit(f"  ^^^^ name collision {digest} {filename} (existing: {row[0]} & {existing_canon[row[0]]}")
                if not exists:
                    newly_hashed.append(row)
                mycsv.writerow(row)
    if canonClobber != None:
        canonClobber.close()
    return newly_hashed

if __name__ == "__main__":
    main("unprocessed/misc3.csv", False, "../todo-gb/shit", False, False, True)
    #main("unprocessed/unloze-css_ze-unique.csv", False, "C:/shared/unloze/css_ze", False, True)
    #main("unprocessed/unloze-css_ze-all.csv", False, "C:/shared/unloze/css_ze", True, False)
    #main("unprocessed/moxx-terabox-unique.csv", False, "F:/terabox/cstrike_all/a", False, True)
    #main("unprocessed/moxx-terabox-all.csv", False, "F:/terabox/cstrike_all/a", True, False)
    #main("unprocessed/moxx-terabox-archives.csv", False, "F:/terabox/cstrike_all/archives", True, False)
