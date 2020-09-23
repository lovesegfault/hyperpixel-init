#include <bcm_host.h>
#include <stdio.h>
#include <sys/mman.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>

void *rawaddr;

void set_function(pin_num, function) {
  volatile uint32_t* fsel = (volatile uint32_t*)(rawaddr + 0x200000);
  uint8_t regnum = pin_num / 10;

  uint8_t pin_shift = (pin_num % 10) * 3;

  fsel[regnum] = (fsel[regnum] & ~(0x7 << pin_shift)) | (function << pin_shift);
}

#define kBCM2708PinmuxIn 0
#define kBCM2708PinmuxOut 1
#define kBCM2708Pinmux_ALT5 2
#define kBCM2708Pinmux_ALT4 3
#define kBCM2708Pinmux_ALT0 4
#define kBCM2708Pinmux_ALT1 5
#define kBCM2708Pinmux_ALT2 6
#define kBCM2708Pinmux_ALT3 7

int main(int argc, char **argv) {
  uint32_t arm_phys = bcm_host_get_peripheral_address();
  printf("arm physical is at 0x%x\n", arm_phys);
  int fd = open("/dev/mem", O_RDWR);
  if (fd < 0) {
    perror("unable to open /dev/mem");
    return 1;
  }
  rawaddr = (void*)mmap(NULL, 16 * 1024 * 1024, PROT_READ | PROT_WRITE, MAP_SHARED, fd, arm_phys);
  if (rawaddr == MAP_FAILED) {
    perror("unable to mmap");
    return 2;
  }
  close(fd);
  volatile uint32_t *fsel = rawaddr + 0x0;

  for (int i=0; i<10; i++) set_function(i, kBCM2708Pinmux_ALT2);
  for (int i=12; i<18; i++) set_function(i, kBCM2708Pinmux_ALT2);
  for (int i=20; i<26; i++) set_function(i, kBCM2708Pinmux_ALT2);
  return 0;
}
