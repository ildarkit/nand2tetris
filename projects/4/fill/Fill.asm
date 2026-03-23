// This file is part of www.nand2tetris.org
// and the book "The Elements of Computing Systems"
// by Nisan and Schocken, MIT Press.
// File name: projects/4/Fill.asm

// Runs an infinite loop that listens to the keyboard input. 
// When a key is pressed (any key), the program blackens the screen,
// i.e. writes "black" in every pixel. When no key is pressed, 
// the screen should be cleared.
  @i
  M=0
  @8192
  D=A
  @n
  M=D
(LOOP)
  @KBD
  D=M
  @BLACK
  D;JNE
  @SCREEN
  D=M
  @WHITE
  D;JNE
  @LOOP
  0;JMP
(BLACK)
  @SCREEN
  D=A
  @i
  A=D+M
  M=-1
  @i
  M=M+1
  D=M
  @n
  D=D-M
  @BLACK
  D;JLT
  @i
  M=0
  @LOOP
  0;JMP
(WHITE)
  @SCREEN
  D=A
  @i
  A=D+M
  M=0
  @i
  M=M+1
  D=M
  @n
  D=D-M
  @WHITE
  D;JLT
  @i
  M=0
  @LOOP
  0;JMP
(END)
  @END
  0;JMP
