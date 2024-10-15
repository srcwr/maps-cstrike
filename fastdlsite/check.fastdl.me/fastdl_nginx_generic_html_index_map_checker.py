import glob, urllib.request, os, csv

target = "https://google.com/maps/"
with urllib.request.urlopen(target) as response:
    htmllines = response.read().decode("utf-8").splitlines()
#with open("shit.html", encoding="utf-8") as f:
#    htmllines = f.read().splitlines()

good = []
for line in htmllines:
    if not ".bsp.bz2" in line:
        continue
    splits = line.strip().split(' ')
    size = splits[-1]
    bsp = splits[1].split('<')[0].split('>')[1] # regex :drool:
    #good.append(f"{size} {bsp}")
    good.append(bsp.split(".bsp.bz2")[0])
    #print(size, bsp)
print(good)

with urllib.request.urlopen("https://main.fastdl.me/maps_index.html.csv") as response:
    csvlines = response.read().decode("utf-8").splitlines()[1:]
lookup = {}
for row in csv.reader(csvlines):
    lookup[row[0]] = int(row[2])

for filename in good:
    if filename.lower() not in lookup:
        print(f"{target}{filename}.bsp.bz2")

