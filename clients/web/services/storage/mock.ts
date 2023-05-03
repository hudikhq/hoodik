export function getItemsForStatusBar() {
  return [
    ...upload.waiting.map((item) => ({ file: item, type: 'upload:waiting' })),
    ...upload.running.map((item) => ({ file: item, type: 'upload:running' })),
    ...upload.done.map((item) => ({ file: item, type: 'upload:done' })),
    ...upload.failed.map((item) => ({ file: item, type: 'upload:failed' })),
    ...download.waiting.map((item) => ({ file: item, type: 'download:waiting' })),
    ...download.running.map((item) => ({ file: item, type: 'download:running' })),
    ...download.done.map((item) => ({ file: item, type: 'download:done' })),
    ...download.failed.map((item) => ({ file: item, type: 'download:failed' }))
  ]
}

export const download = {
  waiting: [
    {
      id: 288,
      user_id: 1,
      is_owner: true,
      encrypted_metadata:
        'NDB8AsyVzdJ6z9En+eMh9gljRZXpmkoLbbKH7KbDjXTHhTm4dsql/ARUrnMeFICC3JVAwgFF4QlNXS53pn8KF37ZAmFxRSB153Fw0pvHAHm+GgcIkjv5MaI/9Q7OUjEwhWDhS7LxnHxy1XaysRHYUES5XZ63iu2Lt9ZHSEiRzqqRkChwf2gmUoH1EMb/orPKIElgNC6z5f83StQzcbvybLnHvUFgjsDiU2gx7qGHRuHow9d5jHMqy6K7YIkUEdkvOCA7vMMBQsqGInv9+W9b0gBUxkSpLolAf1mMXp+mL8CP4/34UPokKfUrN31/N8p3PWaDWdbm5rUssF06oyvUew==',
      name_hash: '905ca6c529cd714afe1fe5c8acdfa9fecb11ebaec7151e1036bc67a2c1de0ac1',
      mime: 'image/jpeg',
      size: 999999,
      chunks: 1,
      chunks_stored: 1,
      file_id: null,
      file_created_at: '2020-06-08T13:25:52',
      created_at: '2023-04-26T20:10:21.043334',
      finished_upload_at: '2023-04-26T20:10:21.608997',
      is_new: false,
      uploaded_chunks: null,
      parent: false,
      metadata: {
        name: '100935283_3052779144799187_3564168066440888320_n.jpg',
        key: '[object Uint8Array]'
      },
      encrypted: false
    }
  ],
  running: [
    {
      id: 205,
      user_id: 1,
      is_owner: true,
      encrypted_metadata:
        'wNuEEoukIxG8xiJxB4eDZzkwgC/jtD8QXtzdxZ4dLhlmWgcLLowMAkL2k8RApre+V6ti4WvyYlCfG+Ptm4cHUQcbDj9f852Cu+tykOrX8nGFuYoj1nsoLnDHlbX1gL33VQp00SFSpWWw37GryGrDU210pXyNxIvJoCHOrugCR7iv+d5ZJg/fwjR5KlYJFrwhpn4tatec4JS8Zid8P/yuvTUo/wuVpeuDbf/u86B/6akzojN9Zpz7NpAJsAP0qKtE3poas/eMYrnesbGBomErz6HUS/iC2PKfSnGZdOBBZBFpXIQSnJ1QzCNN9Pa84aJjlGKUkYHTvdEnb0E9sRbt0w==',
      name_hash: '6b0e9a974af88cf9436505b53e0b2014074070b431924f3adad4af534de0c37a',
      mime: 'video/mp4',
      size: 1664707537,
      chunks: 397,
      chunks_stored: 397,
      file_id: null,
      file_created_at: '2016-01-01T01:00:03',
      created_at: '2023-04-26T07:19:27.179628',
      finished_upload_at: '2023-04-26T07:25:00.676721',
      is_new: false,
      uploaded_chunks: null,
      parent: false,
      metadata: {
        name: '100935283_3052779144799187_3564168066440888320_n.jpg',
        key: '[object Uint8Array]'
      },
      encrypted: false,
      started_download_at: '2023-04-27T13:46:07.561000',
      downloadedBytes: 260046848
    }
  ],
  done: [
    {
      id: 205,
      user_id: 1,
      is_owner: true,
      encrypted_metadata:
        'wNuEEoukIxG8xiJxB4eDZzkwgC/jtD8QXtzdxZ4dLhlmWgcLLowMAkL2k8RApre+V6ti4WvyYlCfG+Ptm4cHUQcbDj9f852Cu+tykOrX8nGFuYoj1nsoLnDHlbX1gL33VQp00SFSpWWw37GryGrDU210pXyNxIvJoCHOrugCR7iv+d5ZJg/fwjR5KlYJFrwhpn4tatec4JS8Zid8P/yuvTUo/wuVpeuDbf/u86B/6akzojN9Zpz7NpAJsAP0qKtE3poas/eMYrnesbGBomErz6HUS/iC2PKfSnGZdOBBZBFpXIQSnJ1QzCNN9Pa84aJjlGKUkYHTvdEnb0E9sRbt0w==',
      name_hash: '6b0e9a974af88cf9436505b53e0b2014074070b431924f3adad4af534de0c37a',
      mime: 'video/mp4',
      size: 1664707537,
      chunks: 397,
      chunks_stored: 397,
      file_id: null,
      file_created_at: '2016-01-01T01:00:03',
      created_at: '2023-04-26T07:19:27.179628',
      finished_upload_at: '2023-04-26T07:25:00.676721',
      is_new: false,
      uploaded_chunks: null,
      parent: false,
      metadata: {
        name: '100935283_3052779144799187_3564168066440888320_n.jpg',
        key: '[object Uint8Array]'
      },
      encrypted: false,
      started_download_at: '2023-04-27T13:46:07.561000',
      downloadedBytes: 1664707537,
      finished_downloading_at: '2023-04-27T13:47:41.275000'
    }
  ],
  failed: [
    {
      id: 292,
      user_id: 1,
      is_owner: true,
      encrypted_metadata:
        'W3kNaWlOcHtXV4qMUyqF/aBZSBzPksI5kavSYgtW/j6pDPokUy7BFV3n9Rb8AmwxpTpjgVWLDvfdLIhokvSV/FWpMwljOfYcYVZsQpcj4k1/T0VeGsRoQGi69K/6Emr/Cq8QixNSgOQetXJvKBDytXz60bE0XzsbdhJxylKG0uvM9GKRusKKBlDzebV27UI9wFGCzrY2WXpDOPOfMpnhat2KvDoJSwtMG7xtrEjGdOPe3Pc89/02gopDLqwxhrL98jlgvfKcUtPpgNJUmE4wmYntDRmW+n5S5Z2w7cmR9M9WZ/KmGs7UznyK+5MBDjYU9haMhZHJAUl4xVYWoDeAnw==',
      name_hash: '4d86ef8edc5d74a0a34cca00048014ca45f9e554e772127a70dfcb1d0d328403',
      mime: 'video/mp4',
      size: 1117550427,
      chunks: 267,
      chunks_stored: 267,
      file_id: null,
      file_created_at: '2020-06-11T09:46:38',
      created_at: '2023-04-27T06:08:59.749824',
      finished_upload_at: '2023-04-27T06:09:50.068427',
      is_new: false,
      uploaded_chunks: null,
      parent: false,
      metadata: {
        name: '100935283_3052779144799187_3564168066440888320_n.jpg',
        key: '[object Uint8Array]'
      },
      encrypted: false,
      started_download_at: '2023-04-27T13:49:56.635000',
      downloadedBytes: 79691776,
      error: { context: 'Failed to fetch', stack: 'TypeError: Failed to fetch' }
    }
  ]
}

export const upload = {
  waiting: [
    {
      id: 310,
      user_id: 1,
      is_owner: true,
      encrypted_metadata:
        'S14ZYbWOBLG4GczKOjjiol47NhfY9f9f4bhePD95W9fRFC2BBm+MmPFPfEQk1IXtQvxQHOneIOA5BM+L+7Uckpt9jFjG8s/Qlrpk2p80w4/3IR3kSU0w+2lZg3ujg+181fuWXlhop4t+9rbHt7UqDMLC+YCNamCgMyc964sJJpQ6bYwOj/U10TV2J/7DWQEaYYdcJezFdtunwMnLUeqKSVxvlZGiHDbWyMuS2Q2WSSpKmN6xmPQblow5UvzVoECy2T3SMTaQvCUM3cRGYUA8TDAarXdtWs4n/Or9jMFsRdW9c+QoBd6S7/qdWqAkm3I+n3qQEqeA6mXmLfIX/SHHIw==',
      name_hash: 'd5f55bb76135caf668a35210b614c30c2019a5faae4f6926deac443154de1361',
      mime: 'video/mp4',
      size: 1916352808,
      chunks: 457,
      chunks_stored: 0,
      file_id: null,
      file_created_at: '2020-04-26T09:44:12',
      created_at: '2023-04-27T13:46:43.791106',
      finished_upload_at: null,
      is_new: true,
      uploaded_chunks: null,
      metadata: { name: 'GH012592.MP4', key: '[object Uint8Array]' },
      file: '[object File]'
    }
  ],
  running: [
    {
      id: 308,
      user_id: 1,
      is_owner: true,
      encrypted_metadata:
        'UGyhHYcwCHjZZf+bLp7h6lSkDBxjTa4LEz1Trwky8VAE0//nWMUZ+S9i33R4EeVmXoq19/ftzLgJwA5KasgUKWIDVkyR9BwFV9T/KpdOT4Y7FIIgEtqynh8liGnsPQqmvG5umsHU9b1CdFR7k98OcEcS+nOYGplapm1lu1QXAO2AKlY1xhJvZDgqag42nMWcX3KhLIgks5GtKVbqEbmzZmtmKPTuM8QtnknsZnbzySKcM4A8ctgvL6nhou+dtdiyzV3rAsljpJRjgML1wQh5FT4xOMYxdvaLDuwJ2BkHOVgiBwViP4tAKxQJrTFtWTIi5G1cV2q7Xv5oZWpN3U3IzQ==',
      name_hash: '41f7cf25048b3a3bd7532ca47b3ffb68b5ddcc27aa2cfb59d7505d41b6241340',
      mime: 'video/mp4',
      size: 1500307347,
      chunks: 358,
      chunks_stored: 14,
      file_id: null,
      file_created_at: '2020-04-26T09:47:42',
      created_at: '2023-04-27T13:44:35.978027',
      finished_upload_at: null,
      is_new: false,
      uploaded_chunks: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14],
      metadata: { name: 'GH012593.MP4', key: '[object Uint8Array]' },
      file: '[object File]',
      started_upload_at: '2023-04-27T13:44:36.559000',
      last_progress_at: '2023-04-27T13:44:39.593000'
    }
  ],
  done: [
    {
      id: 301,
      user_id: 1,
      is_owner: true,
      encrypted_metadata:
        '0gt1beNu6b1bqOEnO+DRprSFbmShbDokwwQzK7jvIWMelSii+LTtEl6pQiTjvamhiManfvivqdwwFH1ZB8EbTfiiYdDXvgBoODQP3rB3IoLG6Vaey8Iw5c3EfYVOCUUblATMMZrWxIPWhDvv0C+Idt36P+d3QLkBzmyUMWFy95Ulm1gb4c0f/YgtjNQVSZou4igHieLZl6xgyM7u53yy/+s+zzetnJcfdwHa6znNtHTP9gUXJhUOtMXUBsM4PbgpbjHaDB76lXKMjoQ3nBu2UkB/3+RB3boPxasKUVYM3XXtpS/gn8CjL1Ia9cCNhTYehg8SYUFzyivLhTtai5rsZg==',
      name_hash: '314732d9caeae2a03f772caef15ac955c44a3dc1f7a203c47ade30795770aef6',
      mime: 'image/heic',
      size: 2050636,
      chunks: 1,
      chunks_stored: 1,
      file_id: null,
      file_created_at: '2021-02-28T16:26:49',
      created_at: '2023-04-27T13:32:12.636579',
      finished_upload_at: '2023-04-27T13:43:58.571000',
      is_new: false,
      metadata: { name: 'IMG_1970.heic', key: '[object Uint8Array]' },
      file: '[object File]',
      started_upload_at: '2023-04-27T13:43:58.559000'
    }
  ],
  failed: [
    {
      id: 308,
      user_id: 1,
      is_owner: true,
      encrypted_metadata:
        'UGyhHYcwCHjZZf+bLp7h6lSkDBxjTa4LEz1Trwky8VAE0//nWMUZ+S9i33R4EeVmXoq19/ftzLgJwA5KasgUKWIDVkyR9BwFV9T/KpdOT4Y7FIIgEtqynh8liGnsPQqmvG5umsHU9b1CdFR7k98OcEcS+nOYGplapm1lu1QXAO2AKlY1xhJvZDgqag42nMWcX3KhLIgks5GtKVbqEbmzZmtmKPTuM8QtnknsZnbzySKcM4A8ctgvL6nhou+dtdiyzV3rAsljpJRjgML1wQh5FT4xOMYxdvaLDuwJ2BkHOVgiBwViP4tAKxQJrTFtWTIi5G1cV2q7Xv5oZWpN3U3IzQ==',
      name_hash: '41f7cf25048b3a3bd7532ca47b3ffb68b5ddcc27aa2cfb59d7505d41b6241340',
      mime: 'video/mp4',
      size: 1500307347,
      chunks: 358,
      chunks_stored: 0,
      file_id: null,
      file_created_at: '2020-04-26T09:47:42',
      created_at: '2023-04-27T13:44:35.978027',
      finished_upload_at: null,
      is_new: true,
      metadata: { name: 'GH012593.MP4', key: '[object Uint8Array]' },
      file: '[object File]',
      started_upload_at: '2023-04-27T13:44:36.559000',
      error: {
        context: 'file_not_found:308',
        stack:
          "Error: Request 'POST http://localhost:4554/api/storage/308?chunk=297&checksum=63563e166692d998f717710573049bfc1c5e8ad207134c89e1c8c6f20370ad64' failed with status 404\n    at Api.make (http://localhost:5173/src/stores/api.ts:157:13)\n    at async uploadChunk (http://localhost:5173/src/stores/storage/workers/chunk.ts:21:22)\n    at async http://localhost:5173/src/stores/storage/workers/file.ts:15:24\n    at async uploadFile (http://localhost:5173/src/stores/storage/workers/file.ts:22:5)\n    at async handleUploadFile (http://localhost:5173/src/sw.ts:37:5)"
      },
      cancel: true
    }
  ]
}
