# Data Sync Tool

Data sync tool aims at providing a convenient way of managing and syncing data sources like Tushare and Quandl. This tool provides automatic data set synchronization. You can configure this tool to adapt to your local storage, such as file system, database, etc.

## Architecture

This tool generally follows the DDD design pattern and emphasizes modular decoupling. It has four layers: presentation, services, entities, and infrastructure. Presentation layer is responsible for user interactions. In this layer, the tool has implemented a cli interface and web interface. Users use either way to interact with this tool. Services contain the main business logic of this tool. Entities encapsulate the core concepts and models of this tool. Infrastructure layer contains modules for external data transfer and data storage.

## Detailed Design

todo