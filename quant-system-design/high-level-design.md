## High Level Module Design

## 基础组件

### 数据组件

DataSource, MarketData

MarketData extends DataSource

## 交易模块

主要目的是实现资产的管理和买卖

### 核心交易模块

Portfolio, Asset, Broker

Portfolio是Asset的容器

Broker负责处理买卖操作

#### 辅助模块

Ledger，负责记录买卖操作和Portfolio的变化

#### 交易系统客户端模块

Client

- Live Paper Trading Client
- Backtest Client
  - Backtest模块通过这个来和交易系统交互
  - 重放一段时间的行情，可以手动交易或者程序交易
- TODO: Real Client

has a portfolio

use a broker

trade assets available on the MarketData

asset must exist on the MarketData

can view asset list and prices on the MarketData

can view trade records and value changes of portfolio

这个模块完成后，可以通过ui查看市场数据，查看账户信息，进行交易买卖。实现了Paper Trading功能。

---

回测系统可以在这个paper trade系统的基础上构建。回测系统的主要目的是检验策略的有效性。

核心模块

Strategy, BacktestEngine

核心功能是通过BackestEngine运行用户Strategy, 调用PaperTrading 模块来交易

跨系统交互用Event/Command实现

调用PaperTrading系统，模拟一段时间的交易。

---

Research

因子开发

核心模块

Factor

利用DataSource计算一个向量，可以用于asset排序

绩效分析模块

核心模块

TradeAnalyzer, FactorAnalyzer

TradeAnalyzer根据Ledger产生的记录，计算各项性能指标, 并进行业绩归因

FactorAnalyzer通过因子计算的值模拟出所选资产的各项性能和风险指标
