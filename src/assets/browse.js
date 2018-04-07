var nodes = document.querySelectorAll('.created')
;[].forEach.call(nodes, function(node) {
    var date = new Date(Number.parseInt(node.textContent, 10))
    node.textContent = date.toLocaleString()
})
