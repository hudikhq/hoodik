;(function () {
  const URL = window.URL || window.webkitURL
  if (!URL) {
    throw new Error('This browser does not support Blob URLs')
  }

  if (!window.Worker) {
    throw new Error('This browser does not support Web Workers')
  }

  class Multithread {
    threads: number
    _queue: any[]
    _queueSize: number
    _activeThreads: number
    _debug: { start: number; end: number; time: number }

    constructor(threads: number) {
      this.threads = Math.max(2, threads | 0)
      this._queue = []
      this._queueSize = 0
      this._activeThreads = 0
      this._debug = {
        start: 0,
        end: 0,
        time: 0
      }
    }

    _worker = {
      JSON: () => {
        // @ts-ignore
        const name = /**/ this.fn /**/
        self.addEventListener('message', function (e) {
          let data = e.data
          let view = new DataView(data)
          let len = data.byteLength
          const str = Array(len)
          for (let i = 0; i < len; i++) {
            str[i] = String.fromCharCode(view.getUint8(i))
          }
          const args = JSON.parse(str.join(''))
          const value = /**/ name /**/
            .apply(/**/ name /**/, args)
          try {
            data = JSON.stringify(value)
          } catch (e) {
            throw new Error('Parallel function must return JSON serializable response')
          }
          len = typeof data === 'undefined' ? 0 : data.length
          const buffer = new ArrayBuffer(len)
          view = new DataView(buffer)
          for (let i = 0; i < len; i++) {
            view.setUint8(i, data.charCodeAt(i) & 255)
          }
          self.postMessage(buffer, [buffer])
          self.close()
        })
      },
      Int32: function () {
        // @ts-ignore
        const name = /**/ this.fn /**/
        self.addEventListener('message', function (e) {
          const data = e.data
          let view = new DataView(data)
          let len = data.byteLength / 4
          const arr = Array(len)
          for (let i = 0; i < len; i++) {
            arr[i] = view.getInt32(i * 4)
          }
          let value = /**/ name /**/
            .apply(/**/ name /**/, arr)
          if (!(value instanceof Array)) {
            value = [value]
          }
          len = value.length
          const buffer = new ArrayBuffer(len * 4)
          view = new DataView(buffer)
          for (let i = 0; i < len; i++) {
            view.setInt32(i * 4, value[i])
          }
          self.postMessage(buffer, [buffer])
          self.close()
        })
      },
      Float64: function () {
        // @ts-ignore
        const name = /*{ func }*/ this.fn /*{ /func }*/
        self.addEventListener('message', function (e) {
          const data = e.data
          let view = new DataView(data)
          let len = data.byteLength / 8
          const arr = Array(len)
          for (let i = 0; i < len; i++) {
            arr[i] = view.getFloat64(i * 8)
          }
          let value = /**/ name /**/
            .apply(/**/ name /**/, arr)
          if (!(value instanceof Array)) {
            value = [value]
          }
          len = value.length
          const buffer = new ArrayBuffer(len * 8)
          view = new DataView(buffer)
          for (let i = 0; i < len; i++) {
            view.setFloat64(i * 8, value[i])
          }
          self.postMessage(buffer, [buffer])
          self.close()
        })
      }
    }

    _encode = {
      JSON: function (args: any) {
        let data
        try {
          data = JSON.stringify(args)
        } catch (e) {
          throw new Error('Arguments provided to parallel function must be JSON serializable')
        }
        const len = data.length
        const buffer = new ArrayBuffer(len)
        const view = new DataView(buffer)
        for (let i = 0; i < len; i++) {
          view.setUint8(i, data.charCodeAt(i) & 255)
        }
        return buffer
      },
      Int32: function (args: any) {
        const len = args.length
        const buffer = new ArrayBuffer(len * 4)
        const view = new DataView(buffer)
        for (let i = 0; i < len; i++) {
          view.setInt32(i * 4, args[i])
        }
        return buffer
      },
      Float64: function (args: any) {
        const len = args.length
        const buffer = new ArrayBuffer(len * 8)
        const view = new DataView(buffer)
        for (let i = 0; i < len; i++) {
          view.setFloat64(i * 8, args[i])
        }
        return buffer
      }
    }

    _decode = {
      JSON: function (data: any) {
        const view = new DataView(data)
        const len = data.byteLength
        const str = Array(len)
        for (let i = 0; i < len; i++) {
          str[i] = String.fromCharCode(view.getUint8(i))
        }
        if (!str.length) {
          return
        } else {
          return JSON.parse(str.join(''))
        }
      },
      Int32: function (data: any) {
        const view = new DataView(data)
        const len = data.byteLength / 4
        const arr = Array(len)
        for (let i = 0; i < len; i++) {
          arr[i] = view.getInt32(i * 4)
        }
        return arr
      },
      Float64: function (data: any) {
        const view = new DataView(data)
        const len = data.byteLength / 8
        const arr = Array(len)
        for (let i = 0; i < len; i++) {
          arr[i] = view.getFloat64(i * 8)
        }
        return arr
      }
    }

    _execute(resource: any, args: any, type: string, callback: (...args: any[]) => void) {
      if (!this._activeThreads) {
        this._debug.start = new Date().valueOf()
      }
      if (this._activeThreads < this.threads) {
        this._activeThreads++
        const t = new Date().valueOf()
        const worker = new Worker(resource)
        const buffer = this._encode[type](args)
        const decode = this._decode[type]
        const self = this
        let listener
        if (type === 'JSON') {
          listener = function (e) {
            callback.call(self, decode(e.data))
            self.ready()
          }
        } else {
          listener = function (e) {
            callback.apply(self, decode(e.data))
            self.ready()
          }
        }
        worker.addEventListener('message', listener)
        worker.postMessage(buffer, [buffer])
      } else {
        this._queueSize++
        this._queue.push([resource, args, type, callback])
      }
    }

    ready() {
      this._activeThreads--
      if (this._queueSize) {
        this._execute.apply(this, this._queue.shift())
        this._queueSize--
      } else if (!this._activeThreads) {
        this._debug.end = new Date().valueOf()
        this._debug.time = this._debug.end - this._debug.start
      }
    }

    _prepare(fn: any, type: string) {
      fn = fn
      let name = fn.name
      const fnStr = fn.toString()
      if (!name) {
        name = '$' + ((Math.random() * 10) | 0)
        while (fnStr.indexOf(name) !== -1) {
          name += (Math.random() * 10) | 0
        }
      }

      const script = this._worker[type]
        .toString()
        .replace(/^.*?[\n\r]+/gi, '')
        .replace(/\}[\s]*$/, '')
        .replace(/\/\*\*\/name\/\*\*\//gi, name)
        .replace(/\/\*\*\/func\/\*\*\//gi, fnStr)

      const resource = URL.createObjectURL(new Blob([script], { type: 'text/javascript' }))

      return resource
    }

    process(fn: any, callback: (...args: any[]) => void) {
      const resource = this._prepare(fn, 'JSON')
      const self = this

      return function () {
        self._execute(resource, [].slice.call(arguments), 'JSON', callback)
      }
    }

    processInt32(fn: any, callback: (...args: any[]) => void) {
      const resource = this._prepare(fn, 'Int32')
      const self = this

      return function () {
        self._execute(resource, [].slice.call(arguments), 'Int32', callback)
      }
    }

    processFloat64(fn: any, callback: (...args: any[]) => void) {
      const resource = this._prepare(fn, 'Float64')
      const self = this

      return function () {
        self._execute(resource, [].slice.call(arguments), 'Float64', callback)
      }
    }
  }

  window['Multithread'] = Multithread
})()
