import pandas as pd
import sqlite3
from rich import print

conn = sqlite3.connect("quote.db")
df = pd.read_sql(
    "select quotation, create_time from market_quotation order by id desc limit 10",
    conn
)
print(df)