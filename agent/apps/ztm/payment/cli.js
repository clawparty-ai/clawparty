export default function ({ api, utils }) {
  return pipeline($=>$
    .onStart(ctx => main(ctx))
  )

  function main({ argv, endpoint }) {
    var buffer = new Data

    function output(str) {
      buffer.push(str)
    }

    function error(err) {
      output('ztm: ')
      output(err.message || err.toString())
      output('\n')
      if (err.stack) {
        output(err.stack)
        output('\n')
      }
    }

    function flush() {
      return [buffer, new StreamEnd]
    }

    try {
      return utils.parseArgv(argv, {
        help: text => Promise.resolve(output(text + '\n')),
        commands: [

          {
            title: 'Send a payment request',
            usage: 'send <username>',
            options: `
              --name        <string>       Product name (required)
              --amount      <number>       Payment amount (required)
              --currency    <string>       Currency code (default: USD)
              --description <string>       Product description
              --url         <string>       Product URL
            `,
            action: (args) => {
              var receiver = args['<username>']
              if (!receiver) throw 'Username is required'
              var name = args['--name']
              if (!name) throw 'Option --name is required'
              var amount = Number.parseFloat(args['--amount'])
              if (Number.isNaN(amount) || amount <= 0) throw 'Option --amount must be a positive number'
              var currency = args['--currency'] || 'USD'
              var description = args['--description'] || ''
              var url = args['--url'] || ''
              var product = { name, description, url }
              return api.createPayment(receiver, product, amount, currency).then(payment => {
                output('Payment created:\n')
                output('  ID:       ' + payment.id + '\n')
                output('  To:       ' + payment.receiver + '\n')
                output('  Product:  ' + payment.product.name + '\n')
                output('  Amount:   ' + payment.amount + ' ' + payment.currency + '\n')
                output('  Status:   ' + payment.status + '\n')
              })
            }
          },

          {
            title: 'List payments',
            usage: 'list',
            action: () => {
              return api.listPayments().then(payments => {
                if (payments.length === 0) {
                  output('No payments found.\n')
                  return
                }
                printTable(payments, {
                  'ID': p => p.id.substring(0, 8),
                  'DIRECTION': p => p.sender === endpoint.id ? '-> ' + p.receiver : '<- ' + p.sender,
                  'PRODUCT': p => p.product.name,
                  'AMOUNT': p => p.amount + ' ' + p.currency,
                  'STATUS': p => p.status,
                })
              })
            }
          },

          {
            title: 'Get payment details',
            usage: 'get <payment-id>',
            action: (args) => {
              var id = args['<payment-id>']
              if (!id) throw 'Payment ID is required'
              return api.getPayment(id).then(payment => {
                if (!payment) {
                  output('Payment not found.\n')
                  return
                }
                output('Payment: ' + payment.id + '\n')
                output('  Sender:    ' + payment.sender + '\n')
                output('  Receiver:  ' + payment.receiver + '\n')
                output('  Product:   ' + payment.product.name + '\n')
                if (payment.product.description) {
                  output('  Desc:      ' + payment.product.description + '\n')
                }
                if (payment.product.url) {
                  output('  URL:       ' + payment.product.url + '\n')
                }
                output('  Amount:    ' + payment.amount + ' ' + payment.currency + '\n')
                output('  Status:    ' + payment.status + '\n')
                output('  Created:   ' + new Date(payment.createdAt).toISOString() + '\n')
                output('  Expires:   ' + new Date(payment.expiresAt).toISOString() + '\n')
                if (payment.proof) {
                  output('  Proof:     ' + payment.proof + '\n')
                }
                if (payment.completedAt) {
                  output('  Completed: ' + new Date(payment.completedAt).toISOString() + '\n')
                }
              })
            }
          },

          {
            title: 'Buy a payment (open payment server and connect)',
            usage: 'buy <payment-id>',
            options: `
              --ep   <endpoint>   Seller endpoint to connect to
            `,
            action: (args) => {
              var id = args['<payment-id>']
              if (!id) throw 'Payment ID is required'
              var ep = args['--ep']
              if (!ep) throw 'Option --ep is required (seller endpoint)'

              output('Opening payment server on ' + ep + '...\n')
              return api.getPayment(id).then(payment => {
                if (!payment) throw 'Payment not found'
                output('Payment: ' + payment.product.name + ' - ' + payment.amount + ' ' + payment.currency + '\n')
                output('Status:  ' + payment.status + '\n')
                output('\nTo complete payment, use the payment GUI or call:\n')
                output('  ztm payment get ' + id + '\n')
              })
            }
          },

          {
            title: 'Cancel a payment',
            usage: 'cancel <payment-id>',
            action: (args) => {
              var id = args['<payment-id>']
              if (!id) throw 'Payment ID is required'
              return api.cancelPayment(id).then(payment => {
                output('Payment ' + payment.id + ' cancelled.\n')
              })
            }
          },
        ]
      }).then(flush).catch(err => {
        error(err)
        return flush()
      })

    } catch (err) {
      error(err)
      return Promise.resolve(flush())
    }

    function printTable(data, columns) {
      var cols = Object.entries(columns)
      var colHeaders = cols.map(i => i[0])
      var colFormats = cols.map(i => i[1])
      var colSizes = colHeaders.map(name => name.length)
      var rows = data.map(row => colFormats.map(
        (format, i) => {
          var v = format(row).toString()
          colSizes[i] = Math.max(colSizes[i], v.length)
          return v
        }
      ))
      colHeaders.forEach((name, i) => output(name.padEnd(colSizes[i]) + '  '))
      output('\n')
      rows.forEach(row => {
        row.forEach((v, i) => output(v.padEnd(colSizes[i]) + '  '))
        output('\n')
      })
    }
  }
}
