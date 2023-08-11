# SPDX-License-Identifier: WTFPL
# Copyright 2023 rtldg <rtldg@protonmail.com>

# why is this a mess? three repos, shit code, and various batch scripts...

from datetime import datetime
import sys
import importlib
import traceback
import logging
import os
import time
from threading import Thread
import shutil
import csv
from pathlib import Path
import requests

maps_hasher = importlib.import_module("maps-hasher")

sys.path.append("../gamebanana-things")
# this is meh... if only I'd .replace('-', '_')...
gamebanana_itemizer = importlib.import_module("gamebanana-itemizer")
gamebanana_pages = importlib.import_module("gamebanana-pages")
import peeker


os.environ["GIT_COMMITTER_NAME"] = "srcwrbot"
os.environ["GIT_COMMITTER_EMAIL"] = "bot@srcwr.com"
WEBHOOKURL = open("secretwebhook").read().strip()
PURGETOKEN = open("secretpurge").read().strip()

def webhook(doping, msg):
    data = {
        "content": ("<&871096527202947094> " if doping else "") + msg,
        "username": "autogb",
        "embeds": [],
    }
    result = requests.post(WEBHOOKURL, json=data)
    try:
        result.raise_for_status()
    except requests.exceptions.HTTPError as err:
        print(err)
    else:
        print("Payload delivered successfully, code {}.".format(result.status_code))

def log_error(error):
    print(error)
    with open("shit2.txt", "a", encoding="utf-8") as shit:
        shit.write(error + "\n")
    webhook(True, error)

def purge_cloudflare_cache():
    data = {"purge_everything": True,}
    result = requests.post("https://api.cloudflare.com/client/v4/zones/1aa75e18589c3649abe7da1eb740bf46/purge_cache", json=data, headers={"Authorization": "Bearer "+PURGETOKEN})
    try:
        result.raise_for_status()
    except requests.exceptions.HTTPError as err:
        print(err)
    else:
        print("Payload delivered successfully, code {}.".format(result.status_code))
    pass

def rsync_hashed():
    os.system("start /wait cmd /c ..\\cwrsync_6.2.7_x64_free\\cwrsync.cmd") # lol... wsl rsync spins forever so....

def maps_cstrike_more(now):
    os.system("start /wait cmd /c ..\\maps-cstrike-more\\auto.cmd "+now) # lol...

def peeker_callback(arg):
    webhook(True, "new download at https://gamebanana.com/mods/"+arg.split('_')[0])

first = True
while True:
    if not first:
        time.sleep(60 * 5)
    first = False

    today = datetime.today()
    now = today.strftime('%Y%m%d%H%M')
    now4csv = today.strftime('%Y-%m-%d %H:%M')

    try:
        gamebanana_pages.main("../gamebanana-things")
    except Exception as e:
        logging.error(traceback.format_exc())
        continue
    gamebanana_itemizer.main("../gamebanana-things")
    os.system("git -C ../gamebanana-things add gamebanana-items")

    try:
        new_items = peeker.main("../gamebanana-things", peeker_callback)
    except Exception as e:
        logging.error(traceback.format_exc())
        log_error("peeker failed... restart me when you can...")
        break
        #continue

    if len(new_items) < 1:
        continue

    os.system(f'git -C ../gamebanana-things commit --author="srcwrbot <bot@srcwr.com>" -m "{now} -automatic gamebanana"')
    #os.system('git -C ../gamebanana-things push originbot")

    #new_items = ["0_0_xbhop_badges.7z"]
    for item in new_items:
        noext = Path(item).stem
        status = os.system(f"7z x -y ../gamebanana-scrape/{item} -o../todo-auto/{noext}")
        if status != 0:
            log_error(f"failed to extract {item}...")

    newly_hashed = maps_hasher.main("unprocessed/gamebanana-x-automatic.csv", True, "../todo-auto", False, False)
    newly_hashed.sort()

    thread_rsync_hashed = Thread(target=rsync_hashed)
    thread_maps_cstrike_more = Thread(target=maps_cstrike_more, args=(now,))
    thread_rsync_hashed.start()
    thread_maps_cstrike_more.start()

    if len(newly_hashed) > 0:
        recently_added = []
        with open("recently_added.csv", newline='', encoding="utf-8") as f:
            recently_added = [line for line in csv.reader(f)]
        recently_added.pop(0) # remove "mapname,filesize,filesize_bz2,sha1,note,recently_added_note,datetime"
        for item in reversed(newly_hashed):
            recently_added.insert(0, item+["automated upload",now4csv])
        with open("recently_added.csv", "w", newline='', encoding="utf-8") as f:
            mycsv = csv.writer(f)
            mycsv.writerow(["mapname","filesize","filesize_bz2","sha1","note","recently_added_note","datetime"])
            for item in recently_added:
                mycsv.writerow(item)

    os.system("git add recently-added.csv unprocessed/gamebanana-automatic.csv")
    os.system(f'git commit --author="srcwrbot <bot@srcwr.com>" -m "{now} - automatic gamebanana"')

    status = os.system("python process.py") # assume no exceptions... lol...
    if status != 0:
        log_error("process.py failed... restart me when you can...")

    thread_rsync_hashed.join()

    if status == 0:
        os.system("start /wait cmd /c ..\\cwrsync_6.2.7_x64_free\\transfer.cmd") # lol...
        purge_cloudflare_cache()
        os.system("git push originbot")

    thread_maps_cstrike_more.join()
    
    if status == 0:
        print("\n\ndone! looping again...")
    else:
        break