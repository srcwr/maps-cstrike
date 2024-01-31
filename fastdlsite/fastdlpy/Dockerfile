FROM python:3.12.1-alpine3.19
WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY main.py .

CMD ["gunicorn", "--workers", "4", "--bind", "0.0.0.0:55155", "main:app"]
