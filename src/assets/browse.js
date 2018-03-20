function fuzzysearch(needle, haystack) {
  var hlen = haystack.length
  var nlen = needle.length
  if (nlen > hlen) {
    return false
  }
  if (nlen === hlen) {
    return needle === haystack
  }
  outer: for (var i = 0, j = 0; i < nlen; i++) {
    var nch = needle.charCodeAt(i)
    while (j < hlen) {
      if (haystack.charCodeAt(j++) === nch) {
        continue outer
      }
    }
    return false
  }
  return true
}

document.querySelector('#filter').addEventListener('keyup', (e) => {
  var query = e.currentTarget.value
  var entries = document.querySelectorAll('tbody tr.entry')
  var len
  for (var i = 0, len = entries.length; i < len; i++) {
  var el = entries[i]
  var filename = el.children[0].innerText.toLowerCase()
  if (fuzzysearch(query, filename)) {
    if (el.style.display !== 'table-row') {
      el.style.display = 'table-row'
    }
  } else {
    if (el.style.display !== 'none') {
      el.style.display = 'none'
    }
  }
}
})
