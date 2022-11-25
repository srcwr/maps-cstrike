import glob

unique = set()
for filename in glob.glob("unprocessed2/*.csv"):
    with open(filename) as f:
        for line in f:
            unique.add(line.lower())
with open("processed/maps.csv", "w") as f:
    for line in sorted(unique):
        f.write(line)
