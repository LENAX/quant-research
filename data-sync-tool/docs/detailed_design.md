# Data Sync Tool

Data sync tool aims at providing a convenient way of managing and syncing data sources like Tushare and Quandl. This tool provides automatic data set synchronization. You can configure this tool to adapt to your local storage, such as file system, database, etc.

## Architecture

This tool generally follows the DDD design pattern and emphasizes modular decoupling. It has four layers: presentation, services, entities, and infrastructure. Presentation layer is responsible for user interactions. In this layer, the tool has implemented a cli interface and web interface. Users use either way to interact with this tool. Services contain the main business logic of this tool. Entities encapsulate the core concepts and models of this tool. Infrastructure layer contains modules for external data transfer and data storage.

![1676612941960](image/detailed_design/1676612941960.png)

## Detailed Design

从controller层面看，数据同步有以下步骤

1. 生成动态请求参数
2. 获取所有未同步的数据源
3. 将动态请求参数传递给同步模块

如果使用之前tsync的设计，那么步骤1、3之间可以用一个公共的数据结构通信

数据流向是1+2 -> 3

这种设计下，数据源只负责管理其所属的metadata，不负责同步和存储工作

同步逻辑由专门的模块完成，可以抽象成一个domain

那么在这个视角下，整个应用可以拆分成3个domain

1. DataSource, 负责描述和管理远程数据源的元数据、本地存储参数和请求参数
2. RequestArgGeneration, 负责存储、管理和生成同步所需的参数
3. RemoteDataSynchronization，负责管理和控制远程数据源的同步


整个同步的流程为

1. query for all unsynced data source
2. for each unsynced data source

   1. generate request args using data source's metadata
   2. synchronize the local data source using information from data source and generated argument
      1. create synchronization task batches using information from data source and generated argument
      2. while not all tasks are completed
         1. pick all available tasks from the wait list using the max concurrent task settings
         2. if no tasks available, poll for each second to see if any task is ready
         3. otherwise, make tasks into http requests, and send the request concurrently using the http client
         4. on request success,
            1. if the request contains no data, discard the request result
            2. otherwise, send the requested data to the message queue for the downstream service to process
3. on request failure,
4. if the server says you requested too frequently,
5. if the wait time is larger than the maximum wait time, then there is no point of waiting, cancel the rest of the task, report an error, and stores a checkpoint
6. otherwise, reset the clock of the task to its cool downtime, and put it back to the wait list to cool down
7. if the server says your day limit is exceeded,
8. report an error,
9. save the checkpoint
10. cancel the rest of the task
11. if the remote says the argument is wrong,
12. report an error
13. drop the task
14. if request time out,
15. report an error
16. retry until max attempt exceeded
17. save the checkpoint
18. cancel synchronization
19. on other error
20. report error
21. save the checkpoint
22. cancel synchronization
