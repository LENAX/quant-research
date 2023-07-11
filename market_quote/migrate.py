import sqlalchemy as sa
from sqlalchemy.sql import text
from sqlalchemy.orm import Session

engine = sa.create_engine("sqlite:///D:\\pprojects\\quant-research\\market_quote\\quote.db")

with open('D:\\pprojects\\quant-research\\market_quote\\migration.sql') as f:
    sql = f.readlines()
    
with engine.connect() as conn:
    for line in sql:
        print(line)
        stmt = text(line)
        conn.execute(stmt)