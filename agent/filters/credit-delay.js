// onSend Filter: credit-delay
// Delays sending based on the peer's current credit.
// credit == BASE_CREDIT → BASE_DELAY_MS
// credit < BASE_CREDIT  → +3000ms per missing credit point
// Returns a Promise that resolves after the delay, then null to continue.
//
// Config (optional):
//   baseCredit   : credit level at which base delay applies (default: 100)
//   baseDelayMs  : base delay in milliseconds (default: 3000)
//   msPerPoint   : extra milliseconds added per missing credit point (default: 3000)

export default function (ctx, config) {
  var baseCredit  = (config && config.baseCredit)  || 100
  var baseDelayMs = (config && config.baseDelayMs) || 3000
  var msPerPoint  = (config && config.msPerPoint)  || 3000

  var credit = ctx.credit
  var diff   = baseCredit - credit
  var delay  = Math.max(0, baseDelayMs + diff * msPerPoint)

  console.info('[send-filter credit-delay] peer:', ctx.peer, 'credit:', credit, '-> delay:', delay, 'ms')

  return new Timeout(delay / 1000).wait().then(() => null)
}
