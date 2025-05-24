# SPDX-License-Identifier: WTFPL
# Copyright 2025 rtldg <rtldg@protonmail.com>

# /// script
# dependencies = [
#   "numpy",
# ]
# ///

import numpy as np
lines = open("processed/main.fastdl.me/69.html.txt", "r", encoding="utf8").read().strip().splitlines()
lengths = np.array([len(line.strip()) for line in lines])
unique, counts = np.unique(lengths, return_counts=True)
print(np.asarray((unique, counts)).T)
