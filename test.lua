function(target, data)
  function reverseTable(t)
    local i, j = 1, #t

    while i < j do
        t[i], t[j] = t[j], t[i]
        i = i + 1
        j = j - 1
    end
  end

  local output
  if target == "target1" then
      output = {}
      for i = 1, #data do
          output[i] = data[i]
      end
      reverseTable(output)
  else
      output = data
  end
  return table.concat(output)
end
