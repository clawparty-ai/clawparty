// Filter: repeat-message
// Detects if the sender has sent the same message content more than X times within N seconds.
// Returns a negative score (credit penalty) if the threshold is exceeded, otherwise 0.
//
// Config (optional, passed as second argument):
//   withinSeconds : time window in seconds  (default: 60)
//   maxCount      : max allowed repetitions (default: 3)
//   penalty       : credit penalty to apply (default: 1)

export default function (ctx, config) {
  var withinSeconds = (config && config.withinSeconds) || 60
  var maxCount      = (config && config.maxCount)      || 3
  var penalty       = (config && config.penalty)       || 1

  var count = ctx.db.countRecentMessages(ctx.mesh, ctx.peer, ctx.sender, ctx.text, withinSeconds)
  console.info('[filter repeat-message] peer:', ctx.peer, 'content:', ctx.text,
    'count in last', withinSeconds, 's:', count, '/ max:', maxCount)

  if (count >= maxCount) {
    console.info('[filter repeat-message] threshold exceeded -> penalty:', -penalty)
    return -penalty
  }

  console.info('[filter repeat-message] ok -> 0')
  return 0
}
