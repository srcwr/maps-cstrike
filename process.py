import glob
import html
import minify_html

unique = set()
for filename in glob.glob("unprocessed/*.csv"):
    with open(filename) as f:
        for line in f:
            unique.add(line.lower())
unique.remove("mapname,filesize,filesize_bz2,sha1\n")
for filename in glob.glob("filters/*.csv"):
    with open(filename) as f:
        for line in f:
            unique.remove(line.lower())

unique_hashes = {}
for line in unique:
    row = line.split(",")
    unique_hashes[row[3].strip()] = int(row[1])

with open("index_top.html", encoding="utf-8") as f:
    index_html = f.read() + """
    <h1>BORN TO DIE</h1>
    <h2>WORLD IS A FUCK</h2>
    <h3>鬼神 Kill Em All {}</h3>
    <h3>I am trash man</h3>
    <h3>{:,} DEAD BYTES</h3>
    <h4>(sorting is slow... you have been warned...)</h4>
    """.format(len(unique_hashes), sum(size for size in unique_hashes.values()))

index_html += """
<table id="list" class="sortable">
<thead>
<tr>
<th style="width:1%">Map name</th>
<th style="width:5%">Hash</th>
<th style="width:5%">Size bsp</th>
<th style="width:5%">Size bz2</th>
</tr>
</thead>
<tbody>
"""

with open("processed/maps.csv", "w") as f:
    f.write("mapname,filesize,filesize_bz2,sha1\n")
    for line in sorted(unique):
        f.write(line)
        row = line.split(",")
        hash = row[3].strip()
        index_html += """
        <tr>
        <td><a href="#">{}</a></td>
        <td><a href="/hashed/{}.bsp.bz2" download>{}</a></td>
        <td>{}</td>
        <td>{}</td>
        </tr>
        """.format(html.escape(row[0]), hash, hash, row[1], row[2])

with open("index_bottom.html", encoding="utf-8") as f:
    index_html += f.read()
    with open("index.html", "w", encoding="utf-8") as h:
        #h.write(index_html)
        h.write(minify_html.minify(index_html, minify_js=True, minify_css=True))
