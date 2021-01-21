const express = require('express')
const app = express()
const port = 3000
const elasticsearch = require('elasticsearch')
const es_client = new elasticsearch.Client({
    host: 'localhost:9200',
    // apiVersion: '7.X', // use the same version of your Elasticsearch instance
});

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