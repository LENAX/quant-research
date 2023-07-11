import easyquotation
import orjson
import sqlite3
import sqlalchemy as sa
from sqlalchemy.sql import text
import orjson
from datetime import datetime, time
from uuid import uuid4
from time import sleep

quotation = easyquotation.use('sina')
market_rest_time_start = time(hour=11, minute=30, second=1)
market_rest_time_end = time(hour=13,minute=59,second=59)
market_close_time = time(hour=15, minute=0, second=1)

engine = sa.create_engine("sqlite:///D:\\pprojects\\quant-research\\market_quote\\quote.db")


# quotation.market_snapshot(prefix=False)
with engine.connect() as conn:
    while True:
        now = datetime.now()
        # market_closed = now.time() <= market_close_time and ( now.time() < market_rest_time_start or now.time() > market_rest_time_end)
        # if market_closed:
        #     print("market closed. sleeping...")
        # while market_closed:
        #     sleep(1)
            
        quote = quotation.market_snapshot(prefix=False)
        conn.execute("""INSERT INTO market_quotation(uuid, quotation, source, create_time) VALUES (?,?,?,?);""", (str(uuid4()), orjson.dumps(quote).decode('utf8'), 'sina', datetime.now()))
        # sql = """insert into market_quotation (uuid, quotation, source, create_time) values"""+ f"""({str(uuid4())}, {orjson.dumps({}).decode('utf8')}, 'sina', {datetime.now().strftime('%Y-%m-%d %H:%M%S')})"""
        # stmt = text(sql)
        # conn.execute(stmt)
        sleep(0.5)