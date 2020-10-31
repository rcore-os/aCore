#include <stddef.h>
#include <string.h>

size_t strlen(const char *s)
{
	const char *a = s;
	for (; *s; s++);
	return s-a;
}
