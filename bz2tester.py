
import bz2
import hashlib
import glob

thing = False
for filename in glob.iglob("../hashed/*.bsp.bz2"):
    """
    if filename.startswith("../hashed\\a1ba"):
        thing = True
    if not thing:
        continue
    """
    #print(filename)
    with bz2.open(filename) as f:
        digest = hashlib.file_digest(f, "sha1").hexdigest()
        if filename != f"../hashed\\{digest}.bsp.bz2":
            print(f"{filename} is fucked!")
            with open("fucked.txt", "a") as fucked:
                fucked.write(f"{digest}\n")
        else:
            print(digest)
