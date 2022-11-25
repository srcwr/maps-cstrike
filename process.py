import glob

unique = set()
for filename in glob.glob("unprocessed/*.csv"):
    with open(filename) as f:
        for line in f:
            unique.add(line.lower())
unique.remove("mapname,filesize,filesize_bz2,sha1\n")
with open("processed/maps.csv", "w") as f:
    f.write("mapname,filesize,filesize_bz2,sha1\n")
    for line in sorted(unique):
        f.write(line)
