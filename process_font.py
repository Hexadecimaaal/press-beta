import sys, json

j = json.load(sys.stdin)

j = dict(filter((lambda p: p[0].isnumeric()), j.items()))

def transpose(arr):
  arr = arr[5:13]
  for i in range(2, 16):
    bitlist = list(map(lambda u: u & (1 << i), arr))
    byte = 0
    for bit in range(8):
      if bitlist[bit]:
        byte = byte + (1 << bit)
    yield byte

def remove_trailing(l):
  while l[-1] == 0:
    l.pop()
  while l[0] == 0:
    l.pop(0)
  return l

j = dict(sorted(map(lambda p: (int(p[0]), remove_trailing(list(transpose(p[1])))), j.items())))

print(list(map(lambda p: chr(p[0]), j.items())))

print(list(map(lambda p: p[1], j.items())))
