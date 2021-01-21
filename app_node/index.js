const express = require('express')
const port = 3000
const elasticsearch = require('elasticsearch')
const beforeExit = require('before-exit')
const cluster = require('cluster')
const os = require('os')
const {
    SHUTDOWN_TIMEOUT = 10 * 1000,
    CLUSTER_MODE = 'true'
} = process.env

const es_client = new elasticsearch.Client({
    host: 'localhost:9200',
    // apiVersion: '7.X', // use the same version of your Elasticsearch instance
});

if (cluster.isMaster && CLUSTER_MODE === 'true') {
    const cpuCount = os.cpus().length

    for (let i = 0; i < cpuCount; i++) {
        cluster.fork()
    }

    beforeExit.do((signal) => {
        Object.values(cluster.workers).map(worker => worker.kill(signal))
        return new Promise((resolve, reject) => {
            setTimeout(() => {
                resolve()
            }, SHUTDOWN_TIMEOUT)
        })
    })
}
else {
    const app = express()
    app.get('/', async (req, res) => {
        let es_resp = (await es_client.search({
            size: 50,
            body: {
                query: {
                    match_all: {}
                }
            }
        }));
        return res.json(es_resp.hits.hits.map((hit) => hit._source))
    })

    app.listen(port, () => {
        console.log(`Example app listening at http://localhost:${port}`)
    })
    beforeExit.do((signal) => {
        console.log(`Shutting down the server due signal '${signal}'...`)
    });
}