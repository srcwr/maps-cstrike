# SPDX-License-Identifier: WTFPL

import bz2
import hashlib
import glob
import shutil

dir = "../todo-gb/nfo scrapes todo"
thing = False
for filename in glob.iglob(dir + "/*.bsp.bz2"):
    """
    if filename.startswith("../hashed\\a1ba"):
        thing = True
    if not thing:
        continue
    """
    print(filename)
    with bz2.open(filename) as f:
        content = f.read()
        if not content.startswith(b'VBSP\x13') and not content.startswith(b'VBSP\x14'):
            print(f"{filename} isn't a version 19 or 20 bsp")
            continue
        if b"info_player_counterterrorist" in content or b"info_player_terrorist" in content:
            print("    HAS CS:S SPAWNS!!!")
            shutil.copy2(filename, dir+"/css")
        """
        digest = hashlib.file_digest(f, "sha1").hexdigest()
        if filename != f"../hashed\\{digest}.bsp.bz2":
            print(f"{filename} is fucked!")
            with open("fucked.txt", "a") as fucked:
                fucked.write(f"{digest}\n")
        else:
            print(digest)
        """
