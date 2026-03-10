// Filter: blocked-keywords
// Checks whether the received message contains any keyword from the blocked_keywords table.
// Returns a negative score (credit penalty) per matched keyword, otherwise 0.
//
// Config (optional, passed as second argument):
//   penalty : credit penalty per matched keyword (default: 1)

export default function (ctx, config) {
  var penalty  = (config && config.penalty) || 1

  var keywords = ctx.db.getBlockedKeywords(ctx.mesh)
  var text     = ctx.text.toLowerCase()
  var matched  = []

  keywords.forEach(function (kw) {
    if (text.indexOf(kw.toLowerCase()) >= 0) {
      matched.push(kw)
    }
  })

  console.info('[filter blocked-keywords] peer:', ctx.peer, 'keywords checked:', keywords.length,
    'matched:', matched.length > 0 ? matched.join(', ') : 'none')

  if (matched.length > 0) {
    var score = -(penalty * matched.length)
    console.info('[filter blocked-keywords] penalty:', score)
    return score
  }

  console.info('[filter blocked-keywords] ok -> 0')
  return 0
}
