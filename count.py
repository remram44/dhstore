iterators = [-1, -1, -1]
count = 0

def reset(idx):
    iterators[idx] = -1

def advance(idx):
    iterators[idx] += 1
    return iterators[idx] < 3

pos = 0
while True:
    if not advance(pos):
        if pos == 0:
            print "done"
            break
        reset(pos)
        pos -= 1
    elif pos < len(iterators) - 1:
        pos += 1
    else:
        print iterators
