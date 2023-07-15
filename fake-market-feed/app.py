from fastapi import FastAPI, WebSocket
import asyncio
from asyncio import sleep
from faker import Faker
import ujson
import easyquotation

quotation = easyquotation.use('sina')

Faker.seed(0)
fake = Faker()

app = FastAPI()
loop = asyncio.get_running_loop()

def get_quote_blocking() -> dict:
    # return quotation.real('300841')
    return quotation.market_snapshot()

@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()
    while True:
        quote = await loop.run_in_executor(None, get_quote_blocking)
        print(quote)
        await websocket.send_text(ujson.dumps(quote, ensure_ascii=False).replace('\'', "\""))
        await sleep(0.5)
        
@app.get('/')
async def hello() -> 'str':
    data = fake.pydict()
    return f"Hello! {data}"