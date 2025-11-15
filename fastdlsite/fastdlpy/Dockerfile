FROM python:trixie
WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY main.py .

CMD ["gunicorn", "--workers", "4", "--bind", "0.0.0.0:55155", "main:app"]
