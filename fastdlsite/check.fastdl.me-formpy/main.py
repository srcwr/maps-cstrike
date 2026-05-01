# SPDX-License-Identifier: WTFPL

import os
import time
from flask import Flask,request,jsonify

app = Flask(__name__)

@app.post(os.environ["WEBHOOKPATH"])
def AAAAA():
    j = request.get_json()
    with open(time.strftime("/data/forms/%Y%m%d_%H%M%S.txt"), "w", encoding="utf-8") as f:
        f.write(j["content"])
    return jsonify({"yip": "pie"}), 200

if __name__ == '__main__':
    app.run(host='127.0.0.1', port=55154)
