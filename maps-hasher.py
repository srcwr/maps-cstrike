
import bz2
import csv
import glob
import hashlib
import mmap
import os
import shutil
from pathlib import Path
from stat import *

csvname = "unprocessed/misc2.csv"
mapsfolder = "../todo"

if os.path.exists(csvname) and os.path.getsize(csvname) > 50:
    raise Exception("DONT OVERWRITE THAT CSV!")

with open(csvname, "w", newline="", encoding="utf-8") as csvfile:
    mycsv = csv.writer(csvfile)
    mycsv.writerow(["mapname","filesize","filesize_bz2","sha1","note"])
    for filename in glob.glob(mapsfolder + "/**/*.bsp", recursive=True):
        statttt = os.stat(filename)
        if S_ISDIR(statttt.st_mode):
            continue
        filesize = statttt.st_size
        if filesize == 0:
            with open("shit.txt", "a", encoding="utf-8") as shit:
                shit.write(f"==== empty file {filename}\n")
            print(f"==== empty file {filename}")
            continue
        with open(filename, "rb") as f:
            mm = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
            vbspver = mm.read(5)
            if vbspver != b'VBSP\x13' and vbspver != b'VBSP\x14':
                thing = ""
                if vbspver == b'VBSP\x15':
                    thing = "==== skipping CS:GO map " + filename
                elif vbspver[:4] == b'\x1E\x00\x00\x00' or vbspver[:4] == b'\x1D\x00\x00\x00':
                    thing = "==== skipping GoldSrc map? " + filename
                else:
                    thing = "==== not a CS:S map? " + filename
                print(thing)
                with open("shit.txt", "a", encoding="utf-8") as shit:
                    shit.write(thing + "\n")
                continue
            mm.seek(0)
            digest = hashlib.sha1(mm).hexdigest()
            print("Hash {} -- {}".format(digest, filename))
            renameto = "../hashed/" + digest + ".bsp"
            exists = os.path.exists(renameto)
            if not exists:
                print("copying new! {} -> {}".format(filename, renameto))
                shutil.copy2(filename, renameto)
                with bz2.open(renameto + ".bz2", "wb") as fbz2:
                    mm.seek(0)
                    unused = fbz2.write(mm)
                shutil.copystat(renameto, renameto+".bz2")
            mm.close()
            filesize_bz2 = os.stat(renameto + ".bz2").st_size
            pp = Path(filename)
            mycsv.writerow([pp.stem,filesize,filesize_bz2,digest,pp.parent])
