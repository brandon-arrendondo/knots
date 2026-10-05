-- probe: a bare loop whose only test is exit when
-- source: McCabe 1976, v(G) = E - N + 2 on the control-flow graph: the bare loop has no predicate,
--         the exit-when test is the one decision
-- expect P mccabe=2
procedure P (N : in out Integer) is
begin
   loop
      N := N + 1;
      exit when N > 10;
   end loop;
end P;
