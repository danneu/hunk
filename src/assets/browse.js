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

document.querySelector('#filter').addEventListener('keyup', function(e) {
  var query = e.currentTarget.value.trim().toLowerCase()
  var entries = document.querySelectorAll('tbody tr.entry td:first-child')
  for (var i = 0, len = entries.length; i < len; i++) {
    var el = entries[i]
    var filename = el.textContent.toLowerCase()
    if (fuzzysearch(query, filename)) {
      el.parentNode.style.display = 'table-row'
    } else {
      el.parentNode.style.display = 'none'
    }
  }
})
