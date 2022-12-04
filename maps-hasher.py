
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

if os.path.exists(csvname):
    raise Exception("DONT OVERWRITE THAT CSV!")

with open(csvname, "w", newline="") as csvfile:
    mycsv = csv.writer(csvfile)
    for filename in glob.glob(mapsfolder + "/**/*.bsp", recursive=True):
        statttt = os.stat(filename)
        if S_ISDIR(statttt.st_mode):
            continue
        filesize = statttt.st_size
        with open(filename, "rb") as f:
            mm = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
            vbspver = mm.read(5)
            if vbspver != b'VBSP\x13' and vbspver != b'VBSP\x14':
                if vbspver == b'VBSP\x15':
                    print("==== skipping CS:GO map " + filename)
                elif vbspver[:4] == b'\x1E\x00\x00\x00' or vbspver[:4] == b'\x1D\x00\x00\x00':
                    print("==== skipping GoldSrc map? " + filename)
                else:
                    print("==== not a CS:S map? " + filename)
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
            mm.close()
            filesize_bz2 = os.stat(renameto + ".bz2").st_size
            mycsv.writerow([Path(filename).stem,filesize,filesize_bz2,digest])
