CC ?= cc
CFLAGS ?= -Wall -Wextra -std=c11

TARGET := ffi-caller

.PHONY: all run clean

all: $(TARGET)

$(TARGET): caller.c math.c math.h
	$(CC) $(CFLAGS) caller.c math.c -o $(TARGET)

run: $(TARGET)
	./$(TARGET)

clean:
	$(RM) $(TARGET) $(TARGET).exe
